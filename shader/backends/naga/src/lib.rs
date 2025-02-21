use core::num::NonZeroU32;
use std::any::Any;

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
  outputs_define: Vec<ShaderStructFieldMetaInfo>,
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
  pub fn new(stage: ShaderStage) -> Self {
    let stage = match stage {
      ShaderStage::Vertex => naga::ShaderStage::Vertex,
      ShaderStage::Fragment => naga::ShaderStage::Fragment,
      ShaderStage::Compute => naga::ShaderStage::Compute,
    };

    let mut module = naga::Module::default();
    let entry = naga::EntryPoint {
      name: ENTRY_POINT_NAME.to_owned(),
      stage,
      early_depth_test: None,
      workgroup_size: [0, 0, 0],
      function: Default::default(),
      workgroup_size_overrides: None,
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

  fn make_global_expression_inner_raw(
    &mut self,
    expr: naga::Expression,
  ) -> naga::Handle<naga::Expression> {
    self.module.global_expressions.append(expr, Span::UNDEFINED)
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

    let naga_ty = match &ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Sized(f) => match f {
          ShaderSizedValueType::Atomic(t) => naga::TypeInner::Atomic(naga::Scalar {
            kind: match t {
              ShaderAtomicValueType::I32 => naga::ScalarKind::Sint,
              ShaderAtomicValueType::U32 => naga::ScalarKind::Uint,
            },
            width: match t {
              ShaderAtomicValueType::I32 => 4,
              ShaderAtomicValueType::U32 => 4,
            },
          }),
          ShaderSizedValueType::Primitive(p) => map_primitive_type(*p),
          ShaderSizedValueType::Struct(st) => {
            name = st.name.to_owned().into();
            gen_struct_define(self, st.clone(), layout)
          }
          ShaderSizedValueType::FixedSizeArray(ty, size) => naga::TypeInner::Array {
            base: self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(*ty.clone())),
              layout,
            ),
            size: naga::ArraySize::Constant(NonZeroU32::new(*size as u32).unwrap()),
            stride: ty.size_of_self(layout.unwrap_or(StructLayoutTarget::Std430)) as u32,
          },
        },
        ShaderValueSingleType::Unsized(ty) => match ty {
          ShaderUnSizedValueType::UnsizedArray(ty) => naga::TypeInner::Array {
            base: self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(*ty.clone())),
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
          multi_sampled,
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
              multi: *multi_sampled,
            },
            TextureSampleType::Depth => naga::ImageClass::Depth { multi: false },
            TextureSampleType::Sint => naga::ImageClass::Sampled {
              kind: naga::ScalarKind::Sint,
              multi: *multi_sampled,
            },
            TextureSampleType::Uint => naga::ImageClass::Sampled {
              kind: naga::ScalarKind::Uint,
              multi: *multi_sampled,
            },
          };
          naga::TypeInner::Image {
            dim,
            arrayed,
            class,
          }
        }
        ShaderValueSingleType::StorageTexture {
          dimension,
          format,
          access,
        } => {
          let (dim, arrayed) = match dimension {
            TextureViewDimension::D1 => (naga::ImageDimension::D1, false),
            TextureViewDimension::D2 => (naga::ImageDimension::D2, false),
            TextureViewDimension::D2Array => (naga::ImageDimension::D2, true),
            TextureViewDimension::D3 => (naga::ImageDimension::D3, false),
            _ => panic!("Unsupported storage texture dimension"),
          };

          let format = match format {
            StorageFormat::R8Unorm => naga::StorageFormat::R8Unorm,
            StorageFormat::R8Snorm => naga::StorageFormat::R8Snorm,
            StorageFormat::R8Uint => naga::StorageFormat::R8Uint,
            StorageFormat::R8Sint => naga::StorageFormat::R8Sint,
            StorageFormat::R16Uint => naga::StorageFormat::R16Uint,
            StorageFormat::R16Sint => naga::StorageFormat::R16Sint,
            StorageFormat::R16Float => naga::StorageFormat::R16Float,
            StorageFormat::Rg8Unorm => naga::StorageFormat::Rg8Unorm,
            StorageFormat::Rg8Snorm => naga::StorageFormat::Rg8Snorm,
            StorageFormat::Rg8Uint => naga::StorageFormat::Rg8Uint,
            StorageFormat::Rg8Sint => naga::StorageFormat::Rg8Sint,
            StorageFormat::R32Uint => naga::StorageFormat::R32Uint,
            StorageFormat::R32Sint => naga::StorageFormat::R32Sint,
            StorageFormat::R32Float => naga::StorageFormat::R32Float,
            StorageFormat::Rg16Uint => naga::StorageFormat::Rg16Uint,
            StorageFormat::Rg16Sint => naga::StorageFormat::Rg16Sint,
            StorageFormat::Rg16Float => naga::StorageFormat::Rg16Float,
            StorageFormat::Rgba8Unorm => naga::StorageFormat::Rgba8Unorm,
            StorageFormat::Rgba8Snorm => naga::StorageFormat::Rgba8Snorm,
            StorageFormat::Rgba8Uint => naga::StorageFormat::Rgba8Uint,
            StorageFormat::Rgba8Sint => naga::StorageFormat::Rgba8Sint,
            StorageFormat::Bgra8Unorm => naga::StorageFormat::Bgra8Unorm,
            StorageFormat::Rgb10a2Uint => naga::StorageFormat::Rgb10a2Uint,
            StorageFormat::Rgb10a2Unorm => naga::StorageFormat::Rgb10a2Unorm,
            StorageFormat::Rg32Uint => naga::StorageFormat::Rg32Uint,
            StorageFormat::Rg32Sint => naga::StorageFormat::Rg32Sint,
            StorageFormat::Rg32Float => naga::StorageFormat::Rg32Float,
            StorageFormat::Rgba16Uint => naga::StorageFormat::Rgba16Uint,
            StorageFormat::Rgba16Sint => naga::StorageFormat::Rgba16Sint,
            StorageFormat::Rgba16Float => naga::StorageFormat::Rgba16Float,
            StorageFormat::Rgba32Uint => naga::StorageFormat::Rgba32Uint,
            StorageFormat::Rgba32Sint => naga::StorageFormat::Rgba32Sint,
            StorageFormat::Rgba32Float => naga::StorageFormat::Rgba32Float,
            StorageFormat::R16Unorm => naga::StorageFormat::R16Unorm,
            StorageFormat::R16Snorm => naga::StorageFormat::R16Snorm,
            StorageFormat::Rg16Unorm => naga::StorageFormat::Rg16Unorm,
            StorageFormat::Rg16Snorm => naga::StorageFormat::Rg16Snorm,
            StorageFormat::Rgba16Unorm => naga::StorageFormat::Rgba16Unorm,
            StorageFormat::Rgba16Snorm => naga::StorageFormat::Rgba16Snorm,
          };

          let access = match access {
            StorageTextureAccess::Load => naga::StorageAccess::LOAD,
            StorageTextureAccess::Store => naga::StorageAccess::STORE,
            StorageTextureAccess::LoadStore => {
              naga::StorageAccess::LOAD | naga::StorageAccess::STORE
            }
          };

          let class = naga::ImageClass::Storage { format, access };

          naga::TypeInner::Image {
            dim,
            arrayed,
            class,
          }
        }
      },
      ShaderValueType::BindingArray { count, ty } => naga::TypeInner::BindingArray {
        base: self.register_ty_impl(ShaderValueType::Single(ty.clone()), layout),
        size: naga::ArraySize::Constant(NonZeroU32::new(*count as u32).unwrap()),
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
    self.outputs_define.push(ShaderStructFieldMetaInfo {
      name,
      ty: ty.clone(),
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
          ShaderBuiltInDecorator::VertexIndex => naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          }),
          ShaderBuiltInDecorator::VertexInstanceIndex => naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          }),
          ShaderBuiltInDecorator::FragFrontFacing => naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Bool,
            width: naga::BOOL_WIDTH,
          }),
          ShaderBuiltInDecorator::FragSampleIndex => naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          }),
          ShaderBuiltInDecorator::FragSampleMask => naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Uint,
            width: 4,
          }),
          ShaderBuiltInDecorator::FragPositionIn => naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            scalar: naga::Scalar {
              kind: naga::ScalarKind::Float,
              width: 4,
            },
          },
          ShaderBuiltInDecorator::VertexPositionOut => naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            scalar: naga::Scalar {
              kind: naga::ScalarKind::Float,
              width: 4,
            },
          },
          ShaderBuiltInDecorator::FragDepth => naga::TypeInner::Scalar(naga::Scalar {
            kind: naga::ScalarKind::Float,
            width: 4,
          }),
          ShaderBuiltInDecorator::CompLocalInvocationId => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            scalar: naga::Scalar {
              kind: naga::ScalarKind::Uint,
              width: 4,
            },
          },
          ShaderBuiltInDecorator::CompGlobalInvocationId => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            scalar: naga::Scalar {
              kind: naga::ScalarKind::Uint,
              width: 4,
            },
          },
          ShaderBuiltInDecorator::CompLocalInvocationIndex => {
            naga::TypeInner::Scalar(naga::Scalar {
              kind: naga::ScalarKind::Uint,
              width: 4,
            })
          }
          ShaderBuiltInDecorator::CompWorkgroupId => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            scalar: naga::Scalar {
              kind: naga::ScalarKind::Uint,
              width: 4,
            },
          },
          ShaderBuiltInDecorator::CompNumWorkgroup => naga::TypeInner::Vector {
            size: naga::VectorSize::Tri,
            scalar: naga::Scalar {
              kind: naga::ScalarKind::Uint,
              width: 4,
            },
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
        let layout = desc.get_buffer_layout();

        let space = desc.get_address_space().unwrap();
        let space = match space {
          AddressSpace::Function => naga::AddressSpace::Function,
          AddressSpace::Private => naga::AddressSpace::Private,
          AddressSpace::WorkGroup => naga::AddressSpace::WorkGroup,
          AddressSpace::Uniform => naga::AddressSpace::Uniform,
          AddressSpace::Storage { writeable } => naga::AddressSpace::Storage {
            access: if writeable {
              naga::StorageAccess::LOAD | naga::StorageAccess::STORE
            } else {
              naga::StorageAccess::LOAD
            },
          },
          AddressSpace::Handle => naga::AddressSpace::Handle,
        };

        let ty = self.register_ty_impl(desc.ty, layout);
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
      ShaderInputNode::UserDefinedIn {
        ty,
        location,
        interpolation,
      } => {
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
            interpolation: interpolation.map(map_interpolation),
            sampling: None,
            second_blend_source: false,
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

  fn define_next_frag_out(&mut self, ty: ShaderSizedValueType) -> ShaderNodeRawHandle {
    assert!(self.block.len() == 1); // we should define input in root scope
    assert!(self.building_fn.len() == 1);

    self.outputs_define.push(ShaderStructFieldMetaInfo {
      name: format!("frag_out_{}", self.outputs_define.len()),
      ty: ty.clone(),
      ty_deco: ShaderFieldDecorator::Location(self.outputs.len(), None).into(),
    });

    let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(ty));
    let r = self.make_local_var(ty);
    let exp = self.get_expression(r);
    self.outputs.push(exp);
    r
  }

  fn define_next_vertex_output(
    &mut self,
    ty: PrimitiveShaderValueType,
    interpolation: Option<ShaderInterpolation>,
  ) -> ShaderNodeRawHandle {
    self.define_out(
      ty,
      format!("vertex_out_{}", self.outputs_define.len()),
      ShaderFieldDecorator::Location(self.outputs.len(), interpolation),
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
        ShaderNodeExpr::Fake => return ShaderNodeRawHandle { handle: 0 },
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
            AtomicFunction::Exchange { compare, .. } => naga::AtomicFunction::Exchange {
              compare: compare.map(|c| {
                comparison = true;
                self.get_expression(c)
              }),
            },
          };

          let primitive = match ty {
            ShaderAtomicValueType::I32 => PrimitiveShaderValueType::Int32,
            ShaderAtomicValueType::U32 => PrimitiveShaderValueType::Uint32,
          };

          let ty = if let AtomicFunction::Exchange { weak: true, .. } = function {
            let scalar_ty = match ty {
              ShaderAtomicValueType::I32 => naga::Scalar::I32,
              ShaderAtomicValueType::U32 => naga::Scalar::U32,
            };
            self.module.generate_predeclared_type(
              naga::PredeclaredType::AtomicCompareExchangeWeakResult(scalar_ty),
            )
          } else {
            self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(
                ShaderSizedValueType::Primitive(primitive),
              )),
              None,
            )
          };

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
            result: Some(r),
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
            ShaderFunctionType::BuiltIn {
              ty: f,
              ty_help_info,
            } => {
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
                ShaderBuiltInFunction::ArrayLength => {
                  break naga::Expression::ArrayLength(self.get_expression(parameters[0]))
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
                ShaderBuiltInFunction::Modf => {
                  let ty_help_info = ty_help_info.unwrap();
                  let size = map_primitive_vec_size(ty_help_info);
                  self
                    .module
                    .generate_predeclared_type(naga::PredeclaredType::ModfResult {
                      size,
                      scalar: naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: ty_help_info.size_of_self(StructLayoutTarget::Packed) as u8,
                      },
                    });

                  naga::MathFunction::Modf
                }
                ShaderBuiltInFunction::Frexp => {
                  let ty_help_info = ty_help_info.unwrap();
                  let size = map_primitive_vec_size(ty_help_info);
                  self
                    .module
                    .generate_predeclared_type(naga::PredeclaredType::FrexpResult {
                      size,
                      scalar: naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: ty_help_info.size_of_self(StructLayoutTarget::Packed) as u8,
                      },
                    });

                  naga::MathFunction::Frexp
                }
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
        ShaderNodeExpr::TextureQuery(texture, info) => naga::Expression::ImageQuery {
          image: self.get_expression(texture),
          query: match info {
            TextureQuery::Size { level } => naga::ImageQuery::Size {
              level: level.map(|v| self.get_expression(v)),
            },
            TextureQuery::NumLevels => naga::ImageQuery::NumLevels,
            TextureQuery::NumLayers => naga::ImageQuery::NumLayers,
            TextureQuery::NumSamples => naga::ImageQuery::NumSamples,
          },
        },
        ShaderNodeExpr::TextureSampling(ShaderTextureSampling {
          texture,
          sampler,
          position,
          array_index,
          level,
          reference,
          offset,
          gather_channel,
        }) => naga::Expression::ImageSample {
          image: self.get_expression(texture),
          sampler: self.get_expression(sampler),
          gather: gather_channel.map(|v| match v {
            GatherChannel::X => naga::SwizzleComponent::X,
            GatherChannel::Y => naga::SwizzleComponent::Y,
            GatherChannel::Z => naga::SwizzleComponent::Z,
            GatherChannel::W => naga::SwizzleComponent::W,
          }),
          coordinate: self.get_expression(position),
          array_index: array_index.map(|index| self.get_expression(index)),
          offset: offset.map(|offset| {
            let ty = self.register_ty_impl(
              ShaderValueType::Single(ShaderValueSingleType::Sized(
                ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec2Int32),
              )),
              None,
            );
            let a = self.make_global_expression_inner_raw(naga::Expression::Literal(
              naga::Literal::I32(offset.x),
            ));
            let b = self.make_global_expression_inner_raw(naga::Expression::Literal(
              naga::Literal::I32(offset.y),
            ));
            self.make_global_expression_inner_raw(naga::Expression::Compose {
              ty,
              components: vec![a, b],
            })
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
        ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
          texture,
          position,
          array_index,
          level,
          sample_index,
        }) => naga::Expression::ImageLoad {
          image: self.get_expression(texture),
          coordinate: self.get_expression(position),
          array_index: array_index.map(|index| self.get_expression(index)),
          level: level.map(|level| self.get_expression(level)),
          sample: sample_index.map(|sample_index| self.get_expression(sample_index)),
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
          let mut components: Vec<_> = parameters.iter().map(|f| self.get_expression(*f)).collect();

          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(target.clone())),
            None,
          );
          if let ShaderSizedValueType::Struct(meta) = &target {
            let extra = self.struct_extra_padding_count.get(&meta.name).unwrap();
            for _ in 0..*extra {
              components.push(
                self.make_expression_inner_raw(naga::Expression::Literal(naga::Literal::U32(0))),
              );
            }
          }

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
              UnaryOperator::LogicalNot => naga::UnaryOperator::LogicalNot,
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
        },
        ShaderNodeExpr::IndexStatic {
          field_index,
          target: struct_node,
        } => naga::Expression::AccessIndex {
          base: self.get_expression(struct_node),
          index: field_index as u32,
        },
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
            PrimitiveShaderValue::Vec2Bool(v) => {
              impl_p!(v, bool, 2, Bool);
            }
            PrimitiveShaderValue::Vec3Bool(v) => {
              impl_p!(v, bool, 3, Bool);
            }
            PrimitiveShaderValue::Vec4Bool(v) => {
              impl_p!(v, bool, 4, Bool);
            }
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

  fn make_zero_val(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
    let ty = self.register_ty_impl(ty, None);
    self.make_expression_inner(naga::Expression::ZeroValue(ty))
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

  fn texture_store(&mut self, store: ShaderTextureStore) {
    let st = naga::Statement::ImageStore {
      image: self.get_expression(store.image),
      coordinate: self.get_expression(store.position),
      array_index: store.array_index.map(|v| self.get_expression(v)),
      value: self.get_expression(store.value),
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
        let ty = ShaderStructMetaInfo {
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
      !self.fn_mapping.contains_key(name.as_ref().unwrap()),
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

  fn build(&mut self) -> (String, Box<dyn Any>) {
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
    BinaryOperator::BitXor => naga::BinaryOperator::ExclusiveOr,
    BinaryOperator::ShiftLeft => naga::BinaryOperator::ShiftLeft,
    BinaryOperator::ShiftRight => naga::BinaryOperator::ShiftRight,
  }
}

fn map_primitive_vec_size(t: PrimitiveShaderValueType) -> Option<naga::VectorSize> {
  match t {
    PrimitiveShaderValueType::Float32 => None,
    PrimitiveShaderValueType::Vec2Float32 => Some(naga::VectorSize::Bi),
    PrimitiveShaderValueType::Vec3Float32 => Some(naga::VectorSize::Tri),
    PrimitiveShaderValueType::Vec4Float32 => Some(naga::VectorSize::Quad),
    _ => unreachable!(),
  }
}

#[rustfmt::skip]
fn map_primitive_type(t: PrimitiveShaderValueType) -> naga::TypeInner {
  use PrimitiveShaderValueType::*;
  use naga::TypeInner::*;
  use naga::VectorSize::*;
  

  match t {
    PrimitiveShaderValueType::Bool => Scalar(naga::Scalar::BOOL),
    Int32 => Scalar(naga::Scalar::I32),
    Uint32 => Scalar(naga::Scalar::U32),
    Float32 => Scalar(naga::Scalar::F32),
    Vec2Bool => Vector { size: Bi, scalar: naga::Scalar::BOOL },
    Vec3Bool => Vector { size: Tri, scalar: naga::Scalar::BOOL },
    Vec4Bool => Vector { size: Quad, scalar: naga::Scalar::BOOL },
    Vec2Float32 => Vector { size: Bi, scalar: naga::Scalar::F32 },
    Vec3Float32 => Vector { size: Tri, scalar: naga::Scalar::F32 },
    Vec4Float32 => Vector { size: Quad, scalar: naga::Scalar::F32 },
    Vec2Uint32 => Vector { size: Bi, scalar: naga::Scalar::U32 },
    Vec3Uint32 => Vector { size: Tri, scalar: naga::Scalar::U32 },
    Vec4Uint32 => Vector { size: Quad, scalar: naga::Scalar::U32 },
    Vec2Int32 => Vector { size: Bi, scalar: naga::Scalar::I32 },
    Vec3Int32 => Vector { size: Tri, scalar: naga::Scalar::I32 },
    Vec4Int32 => Vector { size: Quad, scalar: naga::Scalar::I32} ,
    Mat2Float32 => Matrix { columns: Bi, rows: Bi, scalar: naga::Scalar::F32 },
    Mat3Float32 => Matrix { columns: Tri, rows: Tri, scalar: naga::Scalar::F32 },
    Mat4Float32 => Matrix { columns: Quad, rows: Quad, scalar: naga::Scalar::F32 },
  }
}

fn gen_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: ShaderStructMetaInfo,
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
  let mut members = struct_member(&meta.name, api, &fields, Some(layout));

  let field_size = size_of_struct_sized_fields(&fields, layout);
  let (name, array_ty) = &meta.last_dynamic_array_field;

  members.push(naga::StructMember {
    name: name.to_string().into(),
    ty: api.register_ty_impl(
      ShaderValueType::Single(ShaderValueSingleType::Unsized(
        ShaderUnSizedValueType::UnsizedArray(Box::new(*array_ty.clone())),
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

fn map_interpolation(interpolation: ShaderInterpolation) -> naga::Interpolation {
  match interpolation {
    ShaderInterpolation::Perspective => naga::Interpolation::Perspective,
    ShaderInterpolation::Linear => naga::Interpolation::Linear,
    ShaderInterpolation::Flat => naga::Interpolation::Flat,
  }
}

fn struct_member(
  name: &str,
  api: &mut ShaderAPINagaImpl,
  fields: &[ShaderStructFieldMetaInfo],
  l: Option<StructLayoutTarget>,
) -> Vec<naga::StructMember> {
  let layout = l.unwrap_or(StructLayoutTarget::Std430); // is this ok??

  let mut members = Vec::new();
  let tail_pad = iter_field_start_offset_in_bytes(fields, layout, &mut |field_offset, fty| {
    let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(fty.ty.clone()));
    let ty = api.register_ty_impl(ty, l);

    let binding = fty.ty_deco.map(|deco| match deco {
      ShaderFieldDecorator::BuiltIn(bt) => naga::Binding::BuiltIn(match_built_in(bt)),
      ShaderFieldDecorator::Location(location, interpolation) => naga::Binding::Location {
        location: location as u32,
        interpolation: interpolation.map(map_interpolation),
        sampling: None,
        second_blend_source: false,
      },
    });

    members.push(naga::StructMember {
      name: fty.name.clone().into(),
      ty,
      binding,
      offset: field_offset as u32,
    });
  });

  let mut extra_explicit_padding_count = 0;
  if let Some(TailPaddingInfo {
    start_byte_offset,
    pad_size_in_bytes,
  }) = tail_pad
  {
    assert!(pad_size_in_bytes % 4 == 0); // we assume the minimal type size is 4 bytes.
    let pad_count = pad_size_in_bytes / 4;
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
        offset: (start_byte_offset + i * 4) as u32,
      });
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
    ShaderBuiltInDecorator::CompNumWorkgroup => naga::BuiltIn::NumWorkGroups,
  }
}
