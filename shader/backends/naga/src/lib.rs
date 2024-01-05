use __core::num::NonZeroU32;
use fast_hash_collection::*;
use naga::{Span, StorageAccess};
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
  struct_extra_padding_count: FastHashMap<String, usize>,
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
      ShaderStages::Compute => naga::ShaderStage::Compute,
    };

    let mut module = naga::Module::default();
    let entry = naga::EntryPoint {
      name: ENTRY_POINT_NAME.to_owned(),
      stage,
      early_depth_test: None,
      workgroup_size: [0, 0, 0],
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
      struct_extra_padding_count: Default::default(),
    };

    api.building_fn.push(naga::Function::default());
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
    let needs_pre_emit = expr.needs_pre_emit();
    let handle = self
      .building_fn
      .last_mut()
      .unwrap()
      .expressions
      .append(expr, Span::UNDEFINED);

    // should we merge these expression emits?
    if !needs_pre_emit {
      self.push_top_statement(naga::Statement::Emit(naga::Range::new_from_bounds(
        handle, handle,
      )));
    }

    handle
  }

  fn make_const_lit(&mut self, lit: naga::Literal) -> naga::Handle<naga::Expression> {
    self
      .module
      .const_expressions
      .append(naga::Expression::Literal(lit), Span::UNDEFINED)
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

    let mut name = None;

    let naga_ty = match ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Sized(f) => match f {
          ShaderSizedValueType::Atomic(t) => naga::TypeInner::Atomic {
            kind: match t {
              ShaderAtomicValueType::I32 => naga::ScalarKind::Sint,
              ShaderAtomicValueType::U32 => naga::ScalarKind::Uint,
            },
            width: match t {
              ShaderAtomicValueType::I32 => 4,
              ShaderAtomicValueType::U32 => 4,
            },
          },
          ShaderSizedValueType::Primitive(p) => map_primitive_type(p),
          ShaderSizedValueType::Struct(st) => {
            name = st.name.to_owned().into();
            gen_struct_define(self, st.to_owned(), layout)
          }
          ShaderSizedValueType::FixedSizeArray((ty, size)) => naga::TypeInner::Array {
            base: self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(*ty)),
              layout,
            ),
            size: naga::ArraySize::Constant(NonZeroU32::new(size as u32).unwrap()),
            stride: ty.size_of_self(layout.unwrap_or(StructLayoutTarget::Std430)) as u32,
          },
        },
        ShaderValueSingleType::Unsized(ty) => match ty {
          ShaderUnSizedValueType::UnsizedArray(ty) => naga::TypeInner::Array {
            base: self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(*ty)),
              layout,
            ),
            size: naga::ArraySize::Dynamic,
            stride: ty.size_of_self(layout.unwrap_or(StructLayoutTarget::Std430)) as u32,
          },
          ShaderUnSizedValueType::UnsizedStruct(meta) => {
            name = meta.name.to_owned().into();
            gen_unsized_struct_define(self, meta)
          }
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
    let naga_ty = naga::Type {
      name,
      inner: naga_ty,
    };
    let type_handle = self.module.types.insert(naga_ty, Span::UNDEFINED);
    self.ty_mapping.insert(ty, type_handle);
    type_handle
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

  fn define_out(
    &mut self,
    ty: PrimitiveShaderValueType,
    name: String,
    ty_deco: ShaderFieldDecorator,
  ) -> ShaderNodeRawHandle {
    assert!(self.block.len() == 1); // we should define input in root scope
    assert!(self.building_fn.len() == 1);

    let ty = ShaderSizedValueType::Primitive(ty);
    self.outputs_define.push(ShaderStructFieldMetaInfoOwned {
      name,
      ty,
      ty_deco: Some(ty_deco),
    });

    let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(ty));
    let r = self.make_local_var(ty);
    let exp = self.get_expression(r);
    self.outputs.push(exp);
    r
  }
}

impl ShaderAPI for ShaderAPINagaImpl {
  type Output = Box<dyn core::any::Any>;

  fn set_workgroup_size(&mut self, size: (u32, u32, u32)) {
    self.module.entry_points[0].workgroup_size = [size.0, size.1, size.2]
  }

  fn barrier(&mut self, scope: BarrierScope) {
    let b = match scope {
      BarrierScope::Storage => naga::Barrier::STORAGE,
      BarrierScope::WorkGroup => naga::Barrier::WORK_GROUP,
    };
    self.push_top_statement(naga::Statement::Barrier(b));
  }

  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle {
    assert!(self.building_fn.len() == 1);
    match input {
      ShaderInputNode::BuiltIn(ty) => {
        let bt = match_built_in(ty);

        let ty = match ty {
          ShaderBuiltInDecorator::VertexIndex => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::VertexInstanceIndex => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::FragFrontFacing => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Bool,
            width: naga::BOOL_WIDTH,
          },
          ShaderBuiltInDecorator::FragSampleIndex => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::FragSampleMask => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::FragPositionIn => naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            kind: naga::ScalarKind::Float,
            width: 4,
          },
          ShaderBuiltInDecorator::VertexPositionOut => naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            kind: naga::ScalarKind::Float,
            width: 4,
          },
          ShaderBuiltInDecorator::FragDepth => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Float,
            width: 4,
          },
          ShaderBuiltInDecorator::CompLocalInvocationId => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::CompGlobalInvocationId => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::CompLocalInvocationIndex => naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          },
          ShaderBuiltInDecorator::CompWorkgroupId => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            kind: naga::ScalarKind::Uint,
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
        let space = match desc.get_buffer_layout() {
          Some(StructLayoutTarget::Std140) => naga::AddressSpace::Uniform,
          Some(StructLayoutTarget::Std430) => naga::AddressSpace::Storage {
            access: if desc.writeable_if_storage {
              StorageAccess::all()
            } else {
              StorageAccess::LOAD
            },
          },
          None => naga::AddressSpace::Handle,
        };
        let g = naga::GlobalVariable {
          name: None,
          space,
          binding: naga::ResourceBinding {
            group: bindgroup_index as u32,
            binding: entry_index as u32,
          }
          .into(),
          ty,
          init: None,
        };
        let g = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g));

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        return_handle
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
            interpolation: naga::Interpolation::Perspective.into(),
            sampling: None,
          }
          .into(),
        })
      }
      ShaderInputNode::WorkGroupShared { ty } => {
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(ty)),
          None,
        );
        let g = naga::GlobalVariable {
          name: None,
          space: naga::AddressSpace::WorkGroup,
          binding: None,
          ty,
          init: None,
        };
        let g = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g));

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        return_handle
      }
      ShaderInputNode::Private { ty } => {
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(ty)),
          None,
        );
        let g = naga::GlobalVariable {
          name: None,
          space: naga::AddressSpace::Private,
          binding: None,
          ty,
          init: None,
        };
        let g = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g));

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        return_handle
      }
    }
  }

  fn define_next_frag_out(&mut self) -> ShaderNodeRawHandle {
    assert!(self.block.len() == 1); // we should define input in root scope
    assert!(self.building_fn.len() == 1);

    let ty = ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec4Float32);
    self.outputs_define.push(ShaderStructFieldMetaInfoOwned {
      name: format!("frag_out_{}", self.outputs_define.len()),
      ty,
      ty_deco: ShaderFieldDecorator::Location(self.outputs.len()).into(),
    });

    let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(ty));
    let r = self.make_local_var(ty);
    let exp = self.get_expression(r);
    self.outputs.push(exp);
    r
  }

  fn define_next_vertex_output(&mut self, ty: PrimitiveShaderValueType) -> ShaderNodeRawHandle {
    self.define_out(
      ty,
      format!("vertex_out_{}", self.outputs_define.len()),
      ShaderFieldDecorator::Location(self.outputs.len()),
    )
  }

  fn define_vertex_position_output(&mut self) -> ShaderNodeRawHandle {
    self.define_out(
      PrimitiveShaderValueType::Vec4Float32,
      String::from("vertex_point_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::VertexPositionOut),
    )
  }

  fn define_frag_depth_output(&mut self) -> ShaderNodeRawHandle {
    self.define_out(
      PrimitiveShaderValueType::Float32,
      String::from("frag_depth_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::FragDepth),
    )
  }

  fn make_expression(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle {
    #[allow(clippy::never_loop)] // we here use loop to early exit match block!
    let expr = loop {
      break match expr {
        ShaderNodeExpr::Zeroed { target } => naga::Expression::ZeroValue(self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(target)),
          None,
        )),
        ShaderNodeExpr::AtomicCall {
          ty,
          pointer,
          function,
          value,
        } => {
          let mut comparison = false;
          let fun = match function {
            AtomicFunction::Add => naga::AtomicFunction::Add,
            AtomicFunction::Subtract => naga::AtomicFunction::Subtract,
            AtomicFunction::And => naga::AtomicFunction::And,
            AtomicFunction::ExclusiveOr => naga::AtomicFunction::ExclusiveOr,
            AtomicFunction::InclusiveOr => naga::AtomicFunction::InclusiveOr,
            AtomicFunction::Min => naga::AtomicFunction::Min,
            AtomicFunction::Max => naga::AtomicFunction::Max,
            AtomicFunction::Exchange { compare } => naga::AtomicFunction::Exchange {
              compare: compare.map(|c| {
                comparison = true;
                self.get_expression(c)
              }),
            },
          };

          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Atomic(
              ty,
            ))),
            None,
          );

          // we have to control here not to emit the call exp.
          let r = self.building_fn.last_mut().unwrap().expressions.append(
            naga::Expression::AtomicResult { ty, comparison },
            Span::UNDEFINED,
          );
          let r_handle = self.make_new_handle();
          self.expression_mapping.insert(r_handle, r);

          self.push_top_statement(naga::Statement::Atomic {
            pointer: self.get_expression(pointer),
            fun,
            value: self.get_expression(value),
            result: r,
          });

          return r_handle;
        }
        ShaderNodeExpr::FunctionCall { meta, parameters } => {
          match meta {
            ShaderFunctionType::Custom(meta) => {
              let (fun, _) = *self.fn_mapping.get(&meta.name).unwrap();
              let fun_desc = self.module.functions.try_get(fun).unwrap();
              // todo, currently we do not support function without return value
              assert!(fun_desc.result.is_some());
              // we have to control here not to emit the call exp.
              let r = self
                .building_fn
                .last_mut()
                .unwrap()
                .expressions
                .append(naga::Expression::CallResult(fun), Span::UNDEFINED);
              let r_handle = self.make_new_handle();
              self.expression_mapping.insert(r_handle, r);

              let arguments = parameters.iter().map(|p| self.get_expression(*p)).collect();

              self.push_top_statement(naga::Statement::Call {
                function: fun,
                arguments,
                result: Some(r),
              });

              return r_handle;
            }
            ShaderFunctionType::BuiltIn(f) => {
              let fun = match f {
                ShaderBuiltInFunction::Transpose => naga::MathFunction::Transpose,
                ShaderBuiltInFunction::Normalize => naga::MathFunction::Normalize,
                ShaderBuiltInFunction::Length => naga::MathFunction::Length,
                ShaderBuiltInFunction::Dot => naga::MathFunction::Dot,
                ShaderBuiltInFunction::Cross => naga::MathFunction::Cross,
                ShaderBuiltInFunction::SmoothStep => naga::MathFunction::SmoothStep,
                ShaderBuiltInFunction::Select => {
                  break naga::Expression::Select {
                    condition: self.get_expression(parameters[2]),
                    accept: self.get_expression(parameters[1]),
                    reject: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::Min => naga::MathFunction::Min,
                ShaderBuiltInFunction::Max => naga::MathFunction::Max,
                ShaderBuiltInFunction::Clamp => naga::MathFunction::Clamp,
                ShaderBuiltInFunction::Abs => naga::MathFunction::Abs,
                ShaderBuiltInFunction::Pow => naga::MathFunction::Pow,
                ShaderBuiltInFunction::Saturate => naga::MathFunction::Saturate,
                ShaderBuiltInFunction::All => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::All,
                    argument: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::Any => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::Any,
                    argument: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::IsNan => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::IsNan,
                    argument: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::IsInf => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::IsInf,
                    argument: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::IsFinite => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::IsFinite,
                    argument: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::IsNormal => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::IsNormal,
                    argument: self.get_expression(parameters[0]),
                  }
                }
                ShaderBuiltInFunction::Cos => naga::MathFunction::Cos,
                ShaderBuiltInFunction::Cosh => naga::MathFunction::Cosh,
                ShaderBuiltInFunction::Sin => naga::MathFunction::Sin,
                ShaderBuiltInFunction::Sinh => naga::MathFunction::Sinh,
                ShaderBuiltInFunction::Tan => naga::MathFunction::Tan,
                ShaderBuiltInFunction::Tanh => naga::MathFunction::Tanh,
                ShaderBuiltInFunction::Acos => naga::MathFunction::Acos,
                ShaderBuiltInFunction::Asin => naga::MathFunction::Asin,
                ShaderBuiltInFunction::Atan => naga::MathFunction::Atan,
                ShaderBuiltInFunction::Atan2 => naga::MathFunction::Atan2,
                ShaderBuiltInFunction::Asinh => naga::MathFunction::Asinh,
                ShaderBuiltInFunction::Acosh => naga::MathFunction::Acosh,
                ShaderBuiltInFunction::Atanh => naga::MathFunction::Atanh,
                ShaderBuiltInFunction::Radians => naga::MathFunction::Radians,
                ShaderBuiltInFunction::Degrees => naga::MathFunction::Degrees,
                ShaderBuiltInFunction::Ceil => naga::MathFunction::Ceil,
                ShaderBuiltInFunction::Floor => naga::MathFunction::Floor,
                ShaderBuiltInFunction::Round => naga::MathFunction::Round,
                ShaderBuiltInFunction::Fract => naga::MathFunction::Fract,
                ShaderBuiltInFunction::Trunc => naga::MathFunction::Trunc,
                ShaderBuiltInFunction::Modf => naga::MathFunction::Modf,
                ShaderBuiltInFunction::Frexp => naga::MathFunction::Frexp,
                ShaderBuiltInFunction::Ldexp => naga::MathFunction::Ldexp,
                ShaderBuiltInFunction::Exp => naga::MathFunction::Exp,
                ShaderBuiltInFunction::Exp2 => naga::MathFunction::Exp2,
                ShaderBuiltInFunction::Log => naga::MathFunction::Log,
                ShaderBuiltInFunction::Log2 => naga::MathFunction::Log2,
                ShaderBuiltInFunction::Outer => naga::MathFunction::Outer,
                ShaderBuiltInFunction::Distance => naga::MathFunction::Distance,
                ShaderBuiltInFunction::FaceForward => naga::MathFunction::FaceForward,
                ShaderBuiltInFunction::Reflect => naga::MathFunction::Reflect,
                ShaderBuiltInFunction::Refract => naga::MathFunction::Refract,
                ShaderBuiltInFunction::Sign => naga::MathFunction::Sign,
                ShaderBuiltInFunction::Fma => naga::MathFunction::Fma,
                ShaderBuiltInFunction::Mix => naga::MathFunction::Mix,
                ShaderBuiltInFunction::Step => naga::MathFunction::Step,
                ShaderBuiltInFunction::Sqrt => naga::MathFunction::Sqrt,
                ShaderBuiltInFunction::InverseSqrt => naga::MathFunction::InverseSqrt,
                ShaderBuiltInFunction::Inverse => naga::MathFunction::Inverse,
                ShaderBuiltInFunction::Determinant => naga::MathFunction::Determinant,
                ShaderBuiltInFunction::CountTrailingZeros => naga::MathFunction::CountTrailingZeros,
                ShaderBuiltInFunction::CountLeadingZeros => naga::MathFunction::CountLeadingZeros,
                ShaderBuiltInFunction::CountOneBits => naga::MathFunction::CountOneBits,
                ShaderBuiltInFunction::ReverseBits => naga::MathFunction::ReverseBits,
                ShaderBuiltInFunction::ExtractBits => naga::MathFunction::ExtractBits,
                ShaderBuiltInFunction::InsertBits => naga::MathFunction::InsertBits,
                ShaderBuiltInFunction::FindLsb => naga::MathFunction::FindLsb,
                ShaderBuiltInFunction::FindMsb => naga::MathFunction::FindMsb,
                ShaderBuiltInFunction::Pack4x8snorm => naga::MathFunction::Pack4x8snorm,
                ShaderBuiltInFunction::Pack4x8unorm => naga::MathFunction::Pack4x8unorm,
                ShaderBuiltInFunction::Pack2x16snorm => naga::MathFunction::Pack2x16snorm,
                ShaderBuiltInFunction::Pack2x16unorm => naga::MathFunction::Pack2x16unorm,
                ShaderBuiltInFunction::Pack2x16float => naga::MathFunction::Pack2x16float,
                ShaderBuiltInFunction::Unpack4x8snorm => naga::MathFunction::Unpack4x8snorm,
                ShaderBuiltInFunction::Unpack4x8unorm => naga::MathFunction::Unpack4x8unorm,
                ShaderBuiltInFunction::Unpack2x16snorm => naga::MathFunction::Unpack2x16snorm,
                ShaderBuiltInFunction::Unpack2x16unorm => naga::MathFunction::Unpack2x16unorm,
                ShaderBuiltInFunction::Unpack2x16float => naga::MathFunction::Unpack2x16float,
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
            let a = self.make_const_lit(naga::Literal::I32(offset.x));
            let b = self.make_const_lit(naga::Literal::I32(offset.y));
            self.module.const_expressions.append(
              naga::Expression::Compose {
                ty,
                components: vec![a, b],
              },
              Span::UNDEFINED,
            )
          }),
          level: match level {
            SampleLevel::Auto => naga::SampleLevel::Auto,
            SampleLevel::Zero => naga::SampleLevel::Zero,
            SampleLevel::Exact(e) => naga::SampleLevel::Exact(self.get_expression(e)),
            SampleLevel::Bias(e) => naga::SampleLevel::Bias(self.get_expression(e)),
            SampleLevel::Gradient { x, y } => naga::SampleLevel::Gradient {
              x: self.get_expression(x),
              y: self.get_expression(y),
            },
          },
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
              let index = match ty.chars().next().unwrap() {
                'x' | 'r' => 0,
                'y' | 'g' => 1,
                'z' | 'b' => 2,
                'w' | 'a' => 3,
                _ => panic!("invalid swizzle"),
              };
              break naga::Expression::AccessIndex {
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
        ShaderNodeExpr::Convert {
          source,
          convert_to,
          convert,
        } => naga::Expression::As {
          expr: self.get_expression(source),
          kind: match convert_to {
            ValueKind::Uint => naga::ScalarKind::Uint,
            ValueKind::Int => naga::ScalarKind::Sint,
            ValueKind::Float => naga::ScalarKind::Float,
            ValueKind::Bool => naga::ScalarKind::Bool,
          },
          convert,
        },
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
        ShaderNodeExpr::Derivative { axis, ctrl, source } => {
          let axis = match axis {
            DerivativeAxis::X => naga::DerivativeAxis::X,
            DerivativeAxis::Y => naga::DerivativeAxis::Y,
            DerivativeAxis::Width => naga::DerivativeAxis::Width,
          };
          let ctrl = match ctrl {
            DerivativeControl::Coarse => naga::DerivativeControl::Coarse,
            DerivativeControl::Fine => naga::DerivativeControl::Fine,
            DerivativeControl::None => naga::DerivativeControl::None,
          };
          naga::Expression::Derivative {
            axis,
            ctrl,
            expr: self.get_expression(source),
          }
        }
        ShaderNodeExpr::Operator(op) => match op {
          OperatorNode::Unary { one, operator } => {
            let op = match operator {
              UnaryOperator::LogicalNot => naga::UnaryOperator::Not,
              UnaryOperator::Neg => naga::UnaryOperator::Negate,
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
          OperatorNode::IndexStatic { array, entry } => naga::Expression::AccessIndex {
            base: self.get_expression(array),
            index: entry,
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
          let mut components: Vec<_> = fields.iter().map(|f| self.get_expression(*f)).collect();
          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(
              meta,
            ))),
            None,
          );

          let extra = self.struct_extra_padding_count.get(meta.name).unwrap();
          for _ in 0..*extra {
            components.push(
              self.make_expression_inner_raw(naga::Expression::Literal(naga::Literal::U32(0))),
            );
          }
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

  fn make_local_var(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
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
    // pre check module level
    let (_, ty) = self.block.last().unwrap();
    if let BlockBuildingState::Function = ty {
      // empty output is possible, for example depth only render target
      if self.building_fn.len() == 1 && !self.outputs_define.is_empty() {
        let ty = ShaderStructMetaInfoOwned {
          name: String::from("ModuleOutput"),
          fields: self.outputs_define.clone(),
        };
        let ty = gen_struct_define(self, ty, None);
        let ty = naga::Type {
          name: None,
          inner: ty,
        };
        let ty = self.module.types.insert(ty, Span::UNDEFINED);

        let components = self
          .outputs
          .clone()
          .iter()
          .map(|local| self.make_expression_inner_raw(naga::Expression::Load { pointer: *local }))
          .collect();

        let rt = self.make_expression_inner(naga::Expression::Compose { ty, components });
        self.do_return(rt.into());

        let bf = self.building_fn.last_mut().unwrap();
        bf.result = naga::FunctionResult { ty, binding: None }.into();
      }
    }

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
    // find last if block in the top level statements
    let top_statements = &mut self.block.last_mut().unwrap().0;
    let index = top_statements
      .iter()
      .rev()
      .position(|s| matches!(s, naga::Statement::If { .. }))
      .expect("expect if clause");
    let if_s = top_statements.remove(top_statements.len() - index - 1);

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
    self.push_top_statement(naga::Statement::Kill)
  }

  fn get_fn(&mut self, name: String) -> Option<ShaderUserDefinedFunction> {
    self.fn_mapping.get(&name).map(|v| v.1.clone())
  }

  fn begin_define_fn(&mut self, name: String, return_ty: Option<ShaderValueType>) {
    let name = Some(name);
    if self.building_fn.iter().any(|f| f.name.eq(&name)) {
      panic!("recursive fn definition is not allowed")
    }

    assert!(
      self.fn_mapping.get(name.as_ref().unwrap()).is_none(),
      "function redefinition"
    );

    let f = naga::Function {
      result: return_ty.map(|ty| naga::FunctionResult {
        ty: self.register_ty_impl(ty, None),
        binding: None,
      }),
      name,
      ..Default::default()
    };

    self.building_fn.push(f);
    self
      .block
      .push((Default::default(), BlockBuildingState::Function));
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

  fn build(&mut self) -> (String, Self::Output) {
    self.pop_scope();

    (ENTRY_POINT_NAME.to_owned(), Box::new(self.module.clone()))
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
    BinaryOperator::ShiftLeft => naga::BinaryOperator::ShiftLeft,
    BinaryOperator::ShiftRight => naga::BinaryOperator::ShiftRight,
  }
}

#[rustfmt::skip]
fn map_primitive_type(t: PrimitiveShaderValueType) -> naga::TypeInner {
  use PrimitiveShaderValueType::*;
  use naga::TypeInner::*;
  use naga::ScalarKind::*;
  use naga::VectorSize::*;

  match t {
    PrimitiveShaderValueType::Bool => Scalar { kind: naga::ScalarKind::Bool, width: naga::BOOL_WIDTH },
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

fn gen_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: ShaderStructMetaInfoOwned,
  l: Option<StructLayoutTarget>,
) -> naga::TypeInner {
  let layout = l.unwrap_or(StructLayoutTarget::Std430); // is this ok??

  let members = struct_member(&meta.name, api, &meta.fields, l);

  assert!(!members.is_empty());

  // is this ok??
  let size = meta.size_of_self(layout);

  naga::TypeInner::Struct {
    members,
    span: size as u32,
  }
}

fn gen_unsized_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: &ShaderUnSizedStructMetaInfo,
) -> naga::TypeInner {
  let layout = StructLayoutTarget::Std430;

  let fields: Vec<_> = meta.sized_fields.iter().map(|f| f.to_owned()).collect();
  let mut members = struct_member(meta.name, api, &fields, Some(layout));

  let field_size = size_of_struct_sized_fields(&fields, layout);
  let (name, array_ty) = meta.last_dynamic_array_field;

  members.push(naga::StructMember {
    name: name.to_string().into(),
    ty: api.register_ty_impl(
      ShaderValueType::Single(ShaderValueSingleType::Unsized(
        ShaderUnSizedValueType::UnsizedArray(array_ty),
      )),
      Some(layout),
    ),
    binding: None,
    offset: field_size as u32,
  });

  naga::TypeInner::Struct {
    members,
    span: (field_size + array_ty.size_of_self(layout)) as u32,
  }
}

fn struct_member(
  name: &str,
  api: &mut ShaderAPINagaImpl,
  fields: &[ShaderStructFieldMetaInfoOwned],
  l: Option<StructLayoutTarget>,
) -> Vec<naga::StructMember> {
  let layout = l.unwrap_or(StructLayoutTarget::Std430); // is this ok??

  let mut extra_explicit_padding_count = 0;
  let mut current_byte_used = 0;
  let mut members = Vec::new();
  for (index, ShaderStructFieldMetaInfoOwned { name, ty, ty_deco }) in fields.iter().enumerate() {
    let next_align_requirement = if index + 1 == fields.len() {
      align_of_struct_sized_fields(fields, layout)
    } else {
      fields[index + 1].ty.align_of_self(layout)
    };

    let field_offset = current_byte_used;
    let type_size = ty.size_of_self(layout);

    current_byte_used += type_size;
    let padding_size = align_offset(current_byte_used, next_align_requirement);
    current_byte_used += padding_size;

    let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(*ty));
    let ty = api.register_ty_impl(ty, l);

    let binding = ty_deco.map(|deco| match deco {
      ShaderFieldDecorator::BuiltIn(bt) => naga::Binding::BuiltIn(match_built_in(bt)),
      ShaderFieldDecorator::Location(location) => naga::Binding::Location {
        location: location as u32,
        interpolation: naga::Interpolation::Perspective.into(),
        sampling: None,
      },
    });

    members.push(naga::StructMember {
      name: name.clone().into(),
      ty,
      binding,
      offset: field_offset as u32,
    });

    // 140 struct requires 16 alignment, when the struct used in array, it's size is divisible by
    // 16 but when use struct in struct it is not necessarily divisible by 16. in upper level api
    // (our std140 auto padding macro), we always make sure the size is round up to 16, so we
    // have to solve the struct in struct case.
    //
    // I tried set the naga struct span, but has no effect, so here we add padding manually..
    if l.is_some() && index + 1 == fields.len() && padding_size > 0 {
      assert!(padding_size % 4 == 0); // we assume the minimal type size is 4 bytes.
      let pad_byte_start = field_offset + type_size;
      let pad_count = padding_size / 4;
      // not using array here because I do not want hit anther strange layout issue!
      for i in 0..pad_count {
        let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(
          ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Uint32),
        ));
        let ty = api.register_ty_impl(ty, l);
        extra_explicit_padding_count += 1;
        members.push(naga::StructMember {
          name: format!("tail_padding_{i}").into(),
          ty,
          binding: None,
          offset: (pad_byte_start + i * 4) as u32,
        });
      }
    }
  }

  api
    .struct_extra_padding_count
    .insert(name.to_string(), extra_explicit_padding_count);
  members
}

fn match_built_in(bt: ShaderBuiltInDecorator) -> naga::BuiltIn {
  match bt {
    ShaderBuiltInDecorator::VertexIndex => naga::BuiltIn::VertexIndex,
    ShaderBuiltInDecorator::VertexInstanceIndex => naga::BuiltIn::InstanceIndex,
    ShaderBuiltInDecorator::FragFrontFacing => naga::BuiltIn::FrontFacing,
    ShaderBuiltInDecorator::FragSampleIndex => naga::BuiltIn::SampleIndex,
    ShaderBuiltInDecorator::FragSampleMask => naga::BuiltIn::SampleMask,
    ShaderBuiltInDecorator::FragPositionIn => naga::BuiltIn::Position { invariant: false },
    ShaderBuiltInDecorator::VertexPositionOut => naga::BuiltIn::Position { invariant: false },
    ShaderBuiltInDecorator::FragDepth => naga::BuiltIn::FragDepth,
    ShaderBuiltInDecorator::CompLocalInvocationId => naga::BuiltIn::LocalInvocationId,
    ShaderBuiltInDecorator::CompGlobalInvocationId => naga::BuiltIn::GlobalInvocationId,
    ShaderBuiltInDecorator::CompLocalInvocationIndex => naga::BuiltIn::LocalInvocationIndex,
    ShaderBuiltInDecorator::CompWorkgroupId => naga::BuiltIn::WorkGroupId,
  }
}
