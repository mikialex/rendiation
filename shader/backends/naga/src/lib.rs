#![allow(clippy::field_reassign_with_default)]

use __core::num::NonZeroU32;
use fast_hash_collection::*;
use naga::Span;
use rendiation_shader_api::*;

pub struct ShaderAPINagaImpl {
  module: naga::Module,
  handle_id: usize,
  block: Vec<(Vec<naga::Statement>, BlockBuildingState)>,
  control_structure: Vec<naga::Statement>,
  building_fn: Vec<naga::Function>,
  fn_mapping: FastHashMap<String, (naga::Handle<naga::Function>, ShaderUserDefinedFunction)>,
  ty_mapping: FastHashMap<ShaderValueType, naga::Handle<naga::Type>>,
  expression_mapping: FastHashMap<ShaderNodeRawHandle, naga::Handle<naga::Expression>>,
  outputs_define: Vec<ShaderStructFieldMetaInfoOwned>,
  outputs: Vec<naga::Handle<naga::Expression>>,
}

pub enum BlockBuildingState {
  Common,
  SwitchCase(SwitchCaseCondition),
  Loop,
  IfAccept,
  Else,
  Function,
}

const ENTRY_POINT_NAME: &str = "main";

impl ShaderAPINagaImpl {
  pub fn new(stage: ShaderStages) -> Self {
    let stage = match stage {
      ShaderStages::Vertex => naga::ShaderStage::Vertex,
      ShaderStages::Fragment => naga::ShaderStage::Fragment,
    };

    let mut module = naga::Module::default();
    let entry = naga::EntryPoint {
      name: ENTRY_POINT_NAME.to_owned(),
      stage,
      early_depth_test: None,    // todo expose
      workgroup_size: [0, 0, 0], // todo expose , why naga not make this an enum??
      function: Default::default(),
    };
    module.entry_points.push(entry);

    let mut api = Self {
      module,
      handle_id: 0,
      block: Default::default(),
      building_fn: Default::default(),
      fn_mapping: Default::default(),
      expression_mapping: Default::default(),
      ty_mapping: Default::default(),
      control_structure: Default::default(),
      outputs_define: Default::default(),
      outputs: Default::default(),
    };

    api
      .block
      .push((Default::default(), BlockBuildingState::Function));

    api
  }

  fn push_top_statement(&mut self, st: naga::Statement) {
    self.block.last_mut().unwrap().0.push(st);
  }

  fn make_new_handle(&mut self) -> ShaderNodeRawHandle {
    self.handle_id += 1;
    let handle = self.handle_id;
    ShaderNodeRawHandle { handle }
  }

  fn make_expression_inner_raw(
    &mut self,
    expr: naga::Expression,
  ) -> naga::Handle<naga::Expression> {
    let handle = self
      .building_fn
      .last_mut()
      .unwrap()
      .expressions
      .append(expr, Span::UNDEFINED);

    // should we merge these expression emits?
    self.push_top_statement(naga::Statement::Emit(naga::Range::new_from_bounds(
      handle, handle,
    )));

    handle
  }

  fn make_expression_inner(&mut self, expr: naga::Expression) -> ShaderNodeRawHandle {
    let handle = self.make_expression_inner_raw(expr);
    let return_handle = self.make_new_handle();
    self.expression_mapping.insert(return_handle, handle);
    return_handle
  }

  fn register_ty_impl(
    &mut self,
    ty: ShaderValueType,
    layout: Option<StructLayoutTarget>,
  ) -> naga::Handle<naga::Type> {
    if let Some(handle) = self.ty_mapping.get(&ty) {
      return *handle;
    }
    let ty = match ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Sized(f) => match f {
          ShaderSizedValueType::Primitive(p) => map_primitive_type(p),
          ShaderSizedValueType::Struct(st) => {
            let layout = layout.unwrap();
            let members = st
              .fields
              .iter()
              .map(|field| {
                //
                naga::StructMember {
                  name: String::from(field.name).into(),
                  ty: self.register_ty_impl(ty, None),
                  binding: None,
                  offset: todo!(),
                }
              })
              .collect();

            naga::TypeInner::Struct {
              members,
              span: todo!(),
            }
          }
          ShaderSizedValueType::FixedSizeArray((ty, size)) => naga::TypeInner::Array {
            base: self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(*ty)),
              None,
            ),
            size: naga::ArraySize::Constant(NonZeroU32::new(size as u32).unwrap()),
            stride: todo!(),
          },
        },
        ShaderValueSingleType::Unsized(ty) => match ty {
          ShaderUnSizedValueType::UnsizedArray(ty) => todo!(),
          ShaderUnSizedValueType::UnsizedStruct(meta) => todo!(),
        },
        ShaderValueSingleType::Sampler(sampler) => naga::TypeInner::Sampler {
          comparison: matches!(sampler, SamplerBindingType::Comparison),
        },
        ShaderValueSingleType::Texture {
          dimension,
          sample_type,
        } => {
          let (dim, arrayed) = match dimension {
            TextureViewDimension::D1 => (naga::ImageDimension::D1, false),
            TextureViewDimension::D2 => (naga::ImageDimension::D2, false),
            TextureViewDimension::D2Array => (naga::ImageDimension::D2, true),
            TextureViewDimension::Cube => (naga::ImageDimension::Cube, false),
            TextureViewDimension::CubeArray => (naga::ImageDimension::Cube, true),
            TextureViewDimension::D3 => (naga::ImageDimension::D3, false),
          };
          let class = match sample_type {
            TextureSampleType::Float { .. } => naga::ImageClass::Sampled {
              kind: naga::ScalarKind::Float,
              multi: false,
            },
            TextureSampleType::Depth => naga::ImageClass::Depth { multi: false },
            TextureSampleType::Sint => naga::ImageClass::Sampled {
              kind: naga::ScalarKind::Sint,
              multi: false,
            },
            TextureSampleType::Uint => naga::ImageClass::Sampled {
              kind: naga::ScalarKind::Uint,
              multi: false,
            },
          };
          naga::TypeInner::Image {
            dim,
            arrayed,
            class,
          }
        }
      },
      ShaderValueType::BindingArray { count, ty } => naga::TypeInner::BindingArray {
        base: self.register_ty_impl(ShaderValueType::Single(ty), layout),
        size: naga::ArraySize::Constant(NonZeroU32::new(count as u32).unwrap()),
      },
      ShaderValueType::Never => unreachable!(),
    };
    let ty = naga::Type {
      name: None,
      inner: ty,
    };
    self.module.types.insert(ty, Span::UNDEFINED)
  }

  fn get_expression(&self, handle: ShaderNodeRawHandle) -> naga::Handle<naga::Expression> {
    *self.expression_mapping.get(&handle).unwrap()
  }

  fn add_fn_input_inner(&mut self, input: naga::FunctionArgument) -> ShaderNodeRawHandle {
    let fun = self.building_fn.last_mut().unwrap();
    let idx = fun.arguments.len() as u32;
    fun.arguments.push(input);
    self.make_expression_inner(naga::Expression::FunctionArgument(idx))
  }
}

impl ShaderAPI for ShaderAPINagaImpl {
  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle {
    assert!(self.building_fn.len() == 1);
    match input {
      ShaderInputNode::BuiltIn(ty) => {
        let bt = match ty {
          ShaderBuiltIn::VertexIndexId => naga::BuiltIn::VertexIndex,
          ShaderBuiltIn::VertexInstanceId => naga::BuiltIn::InstanceIndex,
          ShaderBuiltIn::FragmentFrontFacing => naga::BuiltIn::FrontFacing,
          ShaderBuiltIn::FragmentSampleIndex => naga::BuiltIn::SampleIndex,
          ShaderBuiltIn::FragmentSampleMask => naga::BuiltIn::SampleMask,
          ShaderBuiltIn::FragmentNDC => naga::BuiltIn::PointCoord,
        };

        let ty = match ty {
          ShaderBuiltIn::VertexIndexId => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltIn::VertexInstanceId => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltIn::FragmentFrontFacing => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Bool,
            width: 4,
          },
          ShaderBuiltIn::FragmentSampleIndex => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltIn::FragmentSampleMask => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltIn::FragmentNDC => naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            kind: naga::ScalarKind::Float,
            width: 4,
          },
        };
        let ty = naga::Type {
          name: None,
          inner: ty,
        };
        let ty = self.module.types.insert(ty, Span::UNDEFINED);

        self.add_fn_input_inner(naga::FunctionArgument {
          name: None,
          ty,
          binding: naga::Binding::BuiltIn(bt).into(),
        })
      }
      ShaderInputNode::Binding {
        desc,
        bindgroup_index,
        entry_index,
      } => {
        let ty = self.register_ty_impl(desc.ty, desc.get_buffer_layout());
        let g = naga::GlobalVariable {
          name: None,
          space: naga::AddressSpace::Private,
          binding: naga::ResourceBinding {
            group: bindgroup_index as u32,
            binding: entry_index as u32,
          }
          .into(),
          ty,
          init: None,
        };
        let g = self.module.global_variables.append(g, Span::UNDEFINED);
        self.make_expression_inner(naga::Expression::GlobalVariable(g))
      }
      ShaderInputNode::UserDefinedIn { ty, location } => {
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(
            ShaderSizedValueType::Primitive(ty),
          )),
          None,
        );
        self.add_fn_input_inner(naga::FunctionArgument {
          name: None,
          ty,
          binding: naga::Binding::Location {
            location: location as u32,
            interpolation: None,
            sampling: None,
          }
          .into(),
        })
      }
    }
  }

  fn define_frag_out(&mut self) -> ShaderNodeRawHandle {
    assert!(self.block.len() == 1); // we should define input in root scope
    assert!(self.building_fn.len() == 1);

    let ty = ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec4Float32);
    self.outputs_define.push(ShaderStructFieldMetaInfoOwned {
      name: format!("frag_out{}", self.outputs_define.len()),
      ty,
      ty_deco: None,
    });

    let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(ty));
    let r = self.make_var(ty);
    let exp = self.get_expression(r);
    self.outputs.push(exp);
    r
  }

  fn make_expression(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle {
    #[allow(clippy::never_loop)] // we here use loop to early exit match block!
    let expr = loop {
      break match expr {
        ShaderNodeExpr::FunctionCall { meta, parameters } => {
          match meta {
            ShaderFunctionType::Custom(meta) => {
              let (fun, _) = self.fn_mapping.get(&meta.name).unwrap();
              let fun_desc = self.module.functions.try_get(*fun).unwrap();
              assert!(fun_desc.result.is_some()); // todo, currently we do not support function without return value
              naga::Expression::CallResult(*fun)
            }
            ShaderFunctionType::BuiltIn(f) => {
              let fun = match f {
                ShaderBuiltInFunction::MatTranspose => naga::MathFunction::Transpose,
                ShaderBuiltInFunction::Normalize => naga::MathFunction::Normalize,
                ShaderBuiltInFunction::Length => naga::MathFunction::Length,
                ShaderBuiltInFunction::Dot => naga::MathFunction::Dot,
                ShaderBuiltInFunction::Cross => naga::MathFunction::Cross,
                ShaderBuiltInFunction::SmoothStep => naga::MathFunction::SmoothStep,
                ShaderBuiltInFunction::Select => {
                  break naga::Expression::Select {
                    condition: self.get_expression(parameters[0]),
                    accept: self.get_expression(parameters[2]),
                    reject: self.get_expression(parameters[1]),
                  }
                }
                ShaderBuiltInFunction::Min => naga::MathFunction::Min,
                ShaderBuiltInFunction::Max => naga::MathFunction::Max,
                ShaderBuiltInFunction::Clamp => naga::MathFunction::Clamp,
                ShaderBuiltInFunction::Abs => naga::MathFunction::Abs,
                ShaderBuiltInFunction::Pow => naga::MathFunction::Pow,
                ShaderBuiltInFunction::Saturate => naga::MathFunction::Saturate,
              };

              naga::Expression::Math {
                fun,
                arg: self.get_expression(parameters[0]),
                arg1: parameters.get(1).map(|v| self.get_expression(*v)),
                arg2: parameters.get(2).map(|v| self.get_expression(*v)),
                arg3: parameters.get(3).map(|v| self.get_expression(*v)),
              }
            }
          }
        }
        ShaderNodeExpr::TextureSampling {
          texture,
          sampler,
          position,
          index,
          level,
          reference,
          offset,
        } => naga::Expression::ImageSample {
          image: self.get_expression(texture),
          sampler: self.get_expression(sampler),
          gather: None,
          coordinate: self.get_expression(position),
          array_index: index.map(|index| self.get_expression(index)),
          offset: offset.map(|offset| {
            let ty = self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(
                ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec2Int32),
              )),
              None,
            );
            let a = self.module.const_expressions.append(
              naga::Expression::Literal(naga::Literal::I32(offset.x)),
              Span::UNDEFINED,
            );
            let b = self.module.const_expressions.append(
              naga::Expression::Literal(naga::Literal::I32(offset.x)),
              Span::UNDEFINED,
            );
            self.module.const_expressions.append(
              naga::Expression::Compose {
                ty,
                components: vec![a, b],
              },
              Span::UNDEFINED,
            )
          }),
          level: level
            .map(|level| naga::SampleLevel::Exact(self.get_expression(level)))
            .unwrap_or(naga::SampleLevel::Auto),
          depth_ref: reference.map(|r| self.get_expression(r)),
        },
        ShaderNodeExpr::Swizzle { ty, source } => {
          let source = self.get_expression(source);

          fn letter_component(letter: char) -> Option<naga::SwizzleComponent> {
            use naga::SwizzleComponent as Sc;
            match letter {
              'x' | 'r' => Some(Sc::X),
              'y' | 'g' => Some(Sc::Y),
              'z' | 'b' => Some(Sc::Z),
              'w' | 'a' => Some(Sc::W),
              _ => None,
            }
          }

          let size = match ty.len() {
            1 => {
              let idx = match ty.chars().next().unwrap() {
                'x' | 'r' => 0,
                'y' | 'g' => 1,
                'z' | 'b' => 2,
                'w' | 'a' => 3,
                _ => panic!("invalid swizzle"),
              };
              let index =
                self.make_expression_inner_raw(naga::Expression::Literal(naga::Literal::U32(idx)));
              break naga::Expression::Access {
                base: source,
                index,
              };
            }
            2 => naga::VectorSize::Bi,
            3 => naga::VectorSize::Tri,
            4 => naga::VectorSize::Quad,
            _ => panic!("invalid swizzle size"),
          };
          let mut pattern = [naga::SwizzleComponent::X; 4];
          for (comp, ch) in pattern.iter_mut().zip(ty.chars()) {
            *comp = letter_component(ch).unwrap();
          }

          naga::Expression::Swizzle {
            size,
            vector: source,
            pattern,
          }
        }
        ShaderNodeExpr::Compose { target, parameters } => {
          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(
              ShaderSizedValueType::Primitive(target),
            )),
            None,
          );
          let components = parameters.iter().map(|f| self.get_expression(*f)).collect();
          naga::Expression::Compose { ty, components }
        }
        ShaderNodeExpr::Operator(op) => match op {
          OperatorNode::Unary { one, operator } => {
            let op = match operator {
              UnaryOperator::LogicalNot => naga::UnaryOperator::Not,
            };
            naga::Expression::Unary {
              op,
              expr: self.get_expression(one),
            }
          }
          OperatorNode::Binary {
            left,
            right,
            operator,
          } => {
            let left = self.get_expression(left);
            let right = self.get_expression(right);
            let op = map_binary_op(operator);
            naga::Expression::Binary { op, left, right }
          }
          OperatorNode::Index { array, entry } => naga::Expression::Access {
            base: self.get_expression(array),
            index: self.get_expression(entry),
          },
        },
        ShaderNodeExpr::FieldGet {
          field_index,
          struct_node,
        } => naga::Expression::AccessIndex {
          base: self.get_expression(struct_node),
          index: field_index as u32,
        },
        ShaderNodeExpr::StructConstruct { meta, fields } => {
          let components = fields.iter().map(|f| self.get_expression(*f)).collect();
          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(
              meta,
            ))),
            None,
          );
          naga::Expression::Compose { ty, components }
        }
        ShaderNodeExpr::Const { data } => {
          // too funky..
          macro_rules! impl_p {
            ( $input: ident, $r_ty: ty, $array_size: tt, $literal_ty: tt) => {
              let arr: [$r_ty; $array_size] = $input.into();
              let components = arr
                .iter()
                .map(|v| {
                  self.make_expression_inner_raw(naga::Expression::Literal(
                    naga::Literal::$literal_ty(*v),
                  ))
                })
                .collect();
              let ty = self.register_ty_impl(
                ShaderValueType::Single(ShaderValueSingleType::Sized(
                  ShaderSizedValueType::Primitive(data.into()),
                )),
                None,
              );
              let expr = naga::Expression::Compose { ty, components };
              return self.make_expression_inner(expr);
            };
          }

          match data {
            PrimitiveShaderValue::Bool(v) => naga::Expression::Literal(naga::Literal::Bool(v)),
            PrimitiveShaderValue::Uint32(v) => naga::Expression::Literal(naga::Literal::U32(v)),
            PrimitiveShaderValue::Int32(v) => naga::Expression::Literal(naga::Literal::I32(v)),
            PrimitiveShaderValue::Float32(v) => naga::Expression::Literal(naga::Literal::F32(v)),
            PrimitiveShaderValue::Vec2Float32(v) => {
              impl_p!(v, f32, 2, F32);
            }
            PrimitiveShaderValue::Vec3Float32(v) => {
              impl_p!(v, f32, 3, F32);
            }
            PrimitiveShaderValue::Vec4Float32(v) => {
              impl_p!(v, f32, 4, F32);
            }
            PrimitiveShaderValue::Vec2Uint32(v) => {
              impl_p!(v, u32, 2, U32);
            }
            PrimitiveShaderValue::Vec3Uint32(v) => {
              impl_p!(v, u32, 3, U32);
            }
            PrimitiveShaderValue::Vec4Uint32(v) => {
              impl_p!(v, u32, 4, U32);
            }
            PrimitiveShaderValue::Vec2Int32(v) => {
              impl_p!(v, i32, 2, I32);
            }
            PrimitiveShaderValue::Vec3Int32(v) => {
              impl_p!(v, i32, 3, I32);
            }
            PrimitiveShaderValue::Vec4Int32(v) => {
              impl_p!(v, i32, 4, I32);
            }
            PrimitiveShaderValue::Mat2Float32(v) => {
              impl_p!(v, f32, 4, F32);
            }
            PrimitiveShaderValue::Mat3Float32(v) => {
              impl_p!(v, f32, 9, F32);
            }
            PrimitiveShaderValue::Mat4Float32(v) => {
              impl_p!(v, f32, 16, F32);
            }
          }
        }
      };
    };

    self.make_expression_inner(expr)
  }

  fn make_var(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
    let v = naga::LocalVariable {
      name: None,
      ty: self.register_ty_impl(ty, None),
      init: None,
    };
    let var = self
      .building_fn
      .last_mut()
      .unwrap()
      .local_variables
      .append(v, Span::UNDEFINED);

    self.make_expression_inner(naga::Expression::LocalVariable(var))
  }

  fn store(&mut self, source: ShaderNodeRawHandle, target: ShaderNodeRawHandle) {
    let st = naga::Statement::Store {
      pointer: self.get_expression(target),
      value: self.get_expression(source),
    };
    self.push_top_statement(st);
  }

  fn load(&mut self, source: ShaderNodeRawHandle) -> ShaderNodeRawHandle {
    let ex = naga::Expression::Load {
      pointer: self.get_expression(source),
    };
    self.make_expression_inner(ex)
  }

  fn push_scope(&mut self) {
    self
      .block
      .push((Default::default(), BlockBuildingState::Common))
  }

  fn pop_scope(&mut self) {
    let (b, ty) = self.block.pop().unwrap();
    let b = naga::Block::from_vec(b);
    match ty {
      BlockBuildingState::Common => self.push_top_statement(naga::Statement::Block(b)),
      BlockBuildingState::SwitchCase(case) => {
        let switch = self.control_structure.last_mut().unwrap();
        if let naga::Statement::Switch { cases, .. } = switch {
          let value = match case {
            SwitchCaseCondition::U32(v) => naga::SwitchValue::U32(v),
            SwitchCaseCondition::I32(v) => naga::SwitchValue::I32(v),
            SwitchCaseCondition::Default => naga::SwitchValue::Default,
          };
          let case = naga::SwitchCase {
            value,
            body: b,
            fall_through: false,
          };
          cases.push(case)
        } else {
          panic!("expect switch")
        }
      }
      BlockBuildingState::Loop => {
        let mut loop_s = self.control_structure.pop().unwrap();
        if let naga::Statement::Loop { body, .. } = &mut loop_s {
          *body = b;
        } else {
          panic!("expect loop")
        }
        self.push_top_statement(loop_s);
      }
      BlockBuildingState::IfAccept => {
        let mut if_s = self.control_structure.pop().unwrap();
        if let naga::Statement::If { accept, .. } = &mut if_s {
          *accept = b;
        } else {
          panic!("expect if")
        }
        self.push_top_statement(if_s);
      }
      BlockBuildingState::Else => {
        let mut if_s = self.control_structure.pop().unwrap();
        if let naga::Statement::If { reject, .. } = &mut if_s {
          *reject = b;
        } else {
          panic!("expect if")
        }
        self.push_top_statement(if_s);
      }
      BlockBuildingState::Function => {
        // is entry
        if self.building_fn.len() == 1 {
          let mut bf = self.building_fn.pop().unwrap();
          bf.body = b;
          let ty = ShaderStructMetaInfoOwned {
            name: String::from("ModuleOutput"),
            fields: self.outputs_define.clone(),
          };
          let ty = todo!();
          bf.result = naga::FunctionResult { ty, binding: None }.into();

          let components = self
            .outputs
            .iter()
            .map(|local| self.make_expression_inner_raw(naga::Expression::Load { pointer: *local }))
            .collect();

          let rt = self.make_expression_inner(naga::Expression::Compose { ty, components });
          self.do_return(rt.into());

          self.module.entry_points[0].function = bf;
        } else {
          let mut bf = self.building_fn.pop().unwrap();
          bf.body = b;
          let name = bf.name.clone().unwrap();
          let handle = self.module.functions.append(bf, Span::UNDEFINED);
          self
            .fn_mapping
            .insert(name.clone(), (handle, ShaderUserDefinedFunction { name }));
        }
      }
    }
  }

  fn push_if_scope(&mut self, condition: ShaderNodeRawHandle) {
    self
      .block
      .push((Default::default(), BlockBuildingState::IfAccept));
    let if_s = naga::Statement::If {
      condition: self.get_expression(condition),
      accept: Default::default(),
      reject: Default::default(),
    };
    self.control_structure.push(if_s);
  }

  fn push_else_scope(&mut self) {
    let if_s = self.block.last_mut().unwrap().0.pop().unwrap();
    assert!(matches!(if_s, naga::Statement::If { .. }));
    self.control_structure.push(if_s);
    self
      .block
      .push((Default::default(), BlockBuildingState::Else));
  }

  fn push_loop_scope(&mut self) {
    self
      .block
      .push((Default::default(), BlockBuildingState::Loop));
    let loop_s = naga::Statement::Loop {
      body: Default::default(),
      continuing: Default::default(),
      break_if: None,
    };
    self.control_structure.push(loop_s);
  }

  fn do_continue(&mut self) {
    let st = naga::Statement::Continue;
    self.push_top_statement(st);
  }
  fn do_break(&mut self) {
    let st = naga::Statement::Break;
    self.push_top_statement(st);
  }

  fn begin_switch(&mut self, selector: ShaderNodeRawHandle) {
    let selector = self.get_expression(selector);
    let switch = naga::Statement::Switch {
      selector,
      cases: Default::default(),
    };
    self.control_structure.push(switch);
  }

  fn push_switch_case_scope(&mut self, case: SwitchCaseCondition) {
    self
      .block
      .push((Default::default(), BlockBuildingState::SwitchCase(case)));
  }

  fn end_switch(&mut self) {
    let switch = self.control_structure.pop().unwrap();
    assert!(matches!(switch, naga::Statement::Switch { .. }));
    self.push_top_statement(switch);
  }

  fn discard(&mut self) {
    self
      .building_fn
      .last_mut()
      .unwrap()
      .body
      .push(naga::Statement::Kill, Span::UNDEFINED)
  }

  fn get_fn(&mut self, name: String) -> Option<ShaderUserDefinedFunction> {
    self.fn_mapping.get(&name).map(|v| v.1.clone())
  }

  fn begin_define_fn(&mut self, name: String, return_ty: Option<ShaderValueType>) {
    let name = Some(name);
    if self.building_fn.iter().any(|f| f.name.eq(&name)) {
      panic!("recursive fn definition is not allowed")
    }

    self.fn_mapping.remove(name.as_ref().unwrap());
    let mut f = naga::Function::default();
    f.name = name;
    self.building_fn.push(f);
    self
      .block
      .push((Default::default(), BlockBuildingState::Function));

    let mut f = self.building_fn.pop().unwrap();
    f.result = return_ty.map(|ty| naga::FunctionResult {
      ty: self.register_ty_impl(ty, None),
      binding: None,
    });
  }

  fn push_fn_parameter(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
    let ty = self.register_ty_impl(ty, None);
    self.add_fn_input_inner(naga::FunctionArgument {
      name: None,
      ty,
      binding: None,
    })
  }

  fn do_return(&mut self, v: Option<ShaderNodeRawHandle>) {
    let value = v.map(|v| self.get_expression(v));
    self.push_top_statement(naga::Statement::Return { value });
  }

  fn end_fn_define(&mut self) -> ShaderUserDefinedFunction {
    let (_, s) = self.block.last().unwrap();
    let f_name = self.building_fn.last().unwrap().name.clone().unwrap();
    assert!(matches!(s, BlockBuildingState::Function));
    self.pop_scope();
    ShaderUserDefinedFunction { name: f_name }
  }

  fn build(&mut self) -> (String, String) {
    self.pop_scope();

    use naga::back::wgsl;

    // validate the IR
    let info = naga::valid::Validator::new(
      naga::valid::ValidationFlags::all(),
      naga::valid::Capabilities::all(),
    )
    .validate(&self.module)
    .unwrap();

    let wgsl = wgsl::write_string(&self.module, &info, wgsl::WriterFlags::empty()).unwrap();

    (wgsl, ENTRY_POINT_NAME.to_owned())
  }
}

fn map_binary_op(o: BinaryOperator) -> naga::BinaryOperator {
  match o {
    BinaryOperator::Add => naga::BinaryOperator::Add,
    BinaryOperator::Sub => naga::BinaryOperator::Subtract,
    BinaryOperator::Mul => naga::BinaryOperator::Multiply,
    BinaryOperator::Div => naga::BinaryOperator::Divide,
    BinaryOperator::Rem => naga::BinaryOperator::Modulo,
    BinaryOperator::Eq => naga::BinaryOperator::Equal,
    BinaryOperator::NotEq => naga::BinaryOperator::NotEqual,
    BinaryOperator::GreaterThan => naga::BinaryOperator::Greater,
    BinaryOperator::LessThan => naga::BinaryOperator::Less,
    BinaryOperator::GreaterEqualThan => naga::BinaryOperator::GreaterEqual,
    BinaryOperator::LessEqualThan => naga::BinaryOperator::LessEqual,
    BinaryOperator::LogicalOr => naga::BinaryOperator::LogicalOr,
    BinaryOperator::LogicalAnd => naga::BinaryOperator::LogicalAnd,
    BinaryOperator::BitAnd => naga::BinaryOperator::And,
    BinaryOperator::BitOr => naga::BinaryOperator::InclusiveOr,
  }
}

#[rustfmt::skip]
fn map_primitive_type(t: PrimitiveShaderValueType) -> naga::TypeInner {
  use PrimitiveShaderValueType::*;
  use naga::TypeInner::*;
  use naga::ScalarKind::*;
  use naga::VectorSize::*;

  match t {
    PrimitiveShaderValueType::Bool => Scalar { kind: naga::ScalarKind::Bool, width: 4 }, // bool is 4 bytes?
    Int32 => Scalar { kind: Sint, width: 4 },
    Uint32 => Scalar { kind: Uint, width: 4 },
    Float32 => Scalar { kind: Float, width: 4 },
    Vec2Float32 => Vector { size: Bi, kind:  Float, width: 4 },
    Vec3Float32 => Vector { size: Tri, kind:  Float, width: 4 },
    Vec4Float32 => Vector { size: Quad, kind:  Float, width: 4 },
    Vec2Uint32 => Vector { size: Bi, kind:  Uint, width: 4 },
    Vec3Uint32 => Vector { size: Tri, kind:  Uint, width: 4 },
    Vec4Uint32 => Vector { size: Quad, kind:  Uint, width: 4 },
    Vec2Int32 => Vector { size: Bi, kind:  Sint, width: 4 },
    Vec3Int32 => Vector { size: Tri, kind:  Sint, width: 4 },
    Vec4Int32 => Vector { size: Quad, kind:  Sint, width: 4 },
    Mat2Float32 => Matrix { columns: Bi, rows: Bi, width: 4 },
    Mat3Float32 => Matrix { columns: Tri, rows: Tri, width: 4 },
    Mat4Float32 => Matrix { columns: Quad, rows: Quad, width: 4 },
  }
}
