use core::num::NonZeroU32;
use std::any::Any;

use fast_hash_collection::*;
use naga::{MemoryDecorations, RayQueryFunction, Span};
use rendiation_shader_api::*;

mod conv;
use conv::*;

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
  log_build_result: bool,
  global_var_mapping: FastHashMap<ShaderNodeRawHandle, naga::Handle<naga::GlobalVariable>>,
  output_mesh_task_size: Option<ShaderNodeRawHandle>,
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
    let stage = map_stage(stage);

    let mut module = naga::Module::default();
    let entry = naga::EntryPoint {
      name: ENTRY_POINT_NAME.to_owned(),
      stage,
      early_depth_test: None,
      workgroup_size: [0, 0, 0],
      function: Default::default(),
      workgroup_size_overrides: None,
      mesh_info: None,
      task_payload: None,
      incoming_ray_payload: None,
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
      log_build_result: false,
      global_var_mapping: Default::default(),
      output_mesh_task_size: Default::default(),
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
    is_global: bool,
  ) -> naga::Handle<naga::Expression> {
    if is_global {
      self.module.global_expressions.append(expr, Span::UNDEFINED)
    } else {
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
  }

  fn make_expression_inner(&mut self, expr: naga::Expression) -> ShaderNodeRawHandle {
    let handle = self.make_expression_inner_raw(expr, false);
    let return_handle = self.make_new_handle();
    self.expression_mapping.insert(return_handle, handle);
    return_handle
  }

  // root cause: this works around a bug in naga's spirv backend. when a compose
  // expression is const-folded into an OpConstantComposite, the backend flattens
  // nested compose/splat expressions but does not flatten Expression::Constant
  // components, so composing a constant value with scalars (e.g. vec4(v3_const, 1.0))
  // produces an OpConstantComposite whose constituent count does not match the vector
  // size, which spirv-val rejects. naga's own wgsl parser never hits this because it
  // deep-copies a constant's init expression into the function arena whenever the
  // constant is referenced in function code, so we mirror that behavior here.
  fn get_compose_component(
    &mut self,
    handle: ShaderNodeRawHandle,
  ) -> naga::Handle<naga::Expression> {
    let expr = self.get_expression(handle);
    let constant = match self.building_fn.last().unwrap().expressions.try_get(expr) {
      Ok(naga::Expression::Constant(c)) => Some(c),
      _ => None,
    };
    match constant {
      Some(c) => self.inline_constant_value_into_fn(*c),
      None => expr,
    }
  }

  fn inline_constant_value_into_fn(
    &mut self,
    constant: naga::Handle<naga::Constant>,
  ) -> naga::Handle<naga::Expression> {
    let init = self.module.constants[constant].init;
    self.copy_global_expr_into_fn(init)
  }

  fn copy_global_expr_into_fn(
    &mut self,
    handle: naga::Handle<naga::Expression>,
  ) -> naga::Handle<naga::Expression> {
    let expr = self.module.global_expressions[handle].clone();
    match expr {
      naga::Expression::Literal(_) | naga::Expression::ZeroValue(_) => {
        self.make_expression_inner_raw(expr, false)
      }
      naga::Expression::Compose { ty, components } => {
        let components = components
          .iter()
          .map(|c| self.copy_global_expr_into_fn(*c))
          .collect();
        self.make_expression_inner_raw(naga::Expression::Compose { ty, components }, false)
      }
      naga::Expression::Constant(c) => self.copy_global_expr_into_fn(self.module.constants[c].init),
      other => unreachable!("unexpected global expression in constant init: {other:?}"),
    }
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
          ShaderSizedValueType::Atomic(t) => naga::TypeInner::Atomic(map_atomic_scalar(*t)),
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
          let (dim, arrayed) = map_image_dimension(*dimension);
          let class = map_texture_sample_type(*sample_type, *multi_sampled);
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
          if matches!(
            dimension,
            TextureViewDimension::Cube | TextureViewDimension::CubeArray
          ) {
            panic!("Unsupported storage texture dimension");
          }
          let (dim, arrayed) = map_image_dimension(*dimension);
          let format = map_storage_format(*format);
          let access = map_storage_access(*access);

          let class = naga::ImageClass::Storage { format, access };

          naga::TypeInner::Image {
            dim,
            arrayed,
            class,
          }
        }
        &ShaderValueSingleType::AccelerationStructure => naga::TypeInner::AccelerationStructure {
          vertex_return: true,
        },
        &ShaderValueSingleType::RayQuery => naga::TypeInner::RayQuery {
          vertex_return: true,
        },
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

  fn create_primitive_expression(
    &mut self,
    data: PrimitiveShaderValue,
    is_global: bool,
  ) -> naga::Handle<naga::Expression> {
    match data {
      PrimitiveShaderValue::Scalar(v) => self.make_expression_inner_raw(
        naga::Expression::Literal(scalar_value_to_naga_literal(v)),
        is_global,
      ),
      PrimitiveShaderValue::Vector { size, scalar, data } => self.compose_primitive_expression(
        PrimitiveShaderValueType::vector(size, scalar),
        data.iter(),
        is_global,
      ),
      PrimitiveShaderValue::Matrix {
        columns,
        rows,
        scalar,
        data,
      } => {
        // naga requires matrix compose from column vectors
        let column_ty = PrimitiveShaderValueType::vector(rows, scalar);
        let components = data
          .iter()
          .map(|column| self.compose_primitive_expression(column_ty, column.iter(), is_global))
          .collect();
        self.compose_expression(
          PrimitiveShaderValueType::Matrix {
            columns,
            rows,
            scalar,
          },
          components,
          is_global,
        )
      }
    }
  }

  fn compose_primitive_expression<'a>(
    &mut self,
    ty: PrimitiveShaderValueType,
    scalars: impl Iterator<Item = &'a ScalarValue>,
    is_global: bool,
  ) -> naga::Handle<naga::Expression> {
    let components = scalars
      .map(|v| {
        self.make_expression_inner_raw(
          naga::Expression::Literal(scalar_value_to_naga_literal(*v)),
          is_global,
        )
      })
      .collect();
    self.compose_expression(ty, components, is_global)
  }

  fn compose_expression(
    &mut self,
    ty: PrimitiveShaderValueType,
    components: Vec<naga::Handle<naga::Expression>>,
    is_global: bool,
  ) -> naga::Handle<naga::Expression> {
    let ty = self.register_ty_impl(
      ShaderValueType::Single(ShaderValueSingleType::Sized(
        ShaderSizedValueType::Primitive(ty),
      )),
      None,
    );
    let expr = naga::Expression::Compose { ty, components };
    self.make_expression_inner_raw(expr, is_global)
  }

  fn define_const_global_expr_impl(
    &mut self,
    value: ShaderStructFieldInitValue,
    raw_ty: &ShaderSizedValueType,
  ) -> naga::Handle<naga::Expression> {
    match (value, raw_ty) {
      (ShaderStructFieldInitValue::Primitive(init), ShaderSizedValueType::Primitive(_)) => {
        self.create_primitive_expression(init, true)
      }
      (ShaderStructFieldInitValue::Struct(init), ShaderSizedValueType::Struct(meta)) => {
        let mut init: Vec<_> = init
          .iter()
          .zip(meta.fields.iter())
          .map(|(v, f_ty)| self.define_const_global_expr_impl(v.clone(), &f_ty.ty))
          .collect();
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(raw_ty.clone())),
          None,
        );
        let extra = self.struct_extra_padding_count.get(&meta.name).unwrap();
        for _ in 0..*extra {
          init.push(
            self.make_expression_inner_raw(naga::Expression::Literal(naga::Literal::U32(0)), true),
          );
        }
        let expr = naga::Expression::Compose {
          ty,
          components: init,
        };
        self.make_expression_inner_raw(expr, true)
      }
      (ShaderStructFieldInitValue::Array(init), ShaderSizedValueType::FixedSizeArray(f_ty, _)) => {
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(raw_ty.clone())),
          None,
        );
        let init = init
          .iter()
          .map(|v| self.define_const_global_expr_impl(v.clone(), f_ty))
          .collect();

        let expr = naga::Expression::Compose {
          ty,
          components: init,
        };
        self.make_expression_inner_raw(expr, true)
      }
      _ => unreachable!("ty not match"),
    }
  }

  fn define_const_impl(
    &mut self,
    value: ShaderStructFieldInitValue,
    ty: ShaderSizedValueType,
    inlined: bool,
  ) -> naga::Handle<naga::Expression> {
    let global_expr = self.define_const_global_expr_impl(value, &ty);

    let ty = self.register_ty_impl(
      ShaderValueType::Single(ShaderValueSingleType::Sized(ty)),
      None,
    );

    let constant = self.module.constants.append(
      naga::Constant {
        // this name should be set, or naga will inlined the const into function.
        name: if inlined {
          None
        } else {
          Some(format!("const{}", self.module.constants.len()))
        },
        ty,
        init: global_expr,
      },
      Span::UNDEFINED,
    );

    self.make_expression_inner_raw(naga::Expression::Constant(constant), false)
  }
}

impl ShaderAPI for ShaderAPINagaImpl {
  fn log_build_result(&mut self) {
    self.log_build_result = true;
  }

  fn set_workgroup_size(&mut self, size: (u32, u32, u32)) {
    self.module.entry_points[0].workgroup_size = [size.0, size.1, size.2]
  }

  fn barrier(&mut self, scope: BarrierScope) {
    let b = map_barrier(scope);
    self.push_top_statement(naga::Statement::ControlBarrier(b));
  }

  fn define_mesh_info(&mut self, mesh_info: MeshStageInfo) {
    let vertex_output_type = self.register_ty_impl(
      ShaderValueType::Single(ShaderValueSingleType::Sized(mesh_info.vertex_output_type)),
      None,
    );

    let primitive_output_type = self.register_ty_impl(
      ShaderValueType::Single(ShaderValueSingleType::Sized(
        mesh_info.primitive_output_type,
      )),
      None,
    );

    let output_variable = *self
      .global_var_mapping
      .get(&mesh_info.output_variable)
      .unwrap();

    self.module.entry_points[0].mesh_info = Some(naga::MeshStageInfo {
      topology: map_mesh_output_topology(mesh_info.topology),
      max_vertices: mesh_info.max_vertices,
      max_vertices_override: None,
      max_primitives: mesh_info.max_primitives,
      max_primitives_override: None,
      vertex_output_type,
      primitive_output_type,
      output_variable,
    });
  }
  fn define_task_payload_io(&mut self, payload: ShaderNodeRawHandle) {
    let output_variable = *self.global_var_mapping.get(&payload).unwrap();
    self.module.entry_points[0].task_payload = Some(output_variable);
  }

  fn set_output_mesh_task_size(&mut self, size: ShaderNodeRawHandle) {
    self.output_mesh_task_size = Some(size);
  }

  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle {
    assert!(self.building_fn.len() == 1);
    match input {
      ShaderInputNode::BuiltIn(ty) => {
        let data_ty = ty
          .data_ty()
          .expect("mesh output relative should defined by shared var");
        let data_ty = ShaderValueType::Single(ShaderValueSingleType::Sized(
          ShaderSizedValueType::Primitive(data_ty),
        ));

        let bt = map_built_in(ty);

        let ty = self.register_ty_impl(data_ty, None);

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
        let space = map_address_space(space);

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
          memory_decorations: MemoryDecorations::empty(),
        };
        let g_h = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g_h), false);

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        self.global_var_mapping.insert(return_handle, g_h);
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
            blend_src: None,
            per_primitive: false,
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
          memory_decorations: MemoryDecorations::empty(),
        };
        let g_h = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g_h), false);

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        self.global_var_mapping.insert(return_handle, g_h);
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
          memory_decorations: MemoryDecorations::empty(),
        };
        let g_h = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g_h), false);

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        self.global_var_mapping.insert(return_handle, g_h);
        return_handle
      }
      ShaderInputNode::TaskPayload { ty } => {
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(ty)),
          None,
        );
        let g = naga::GlobalVariable {
          name: None,
          space: naga::AddressSpace::TaskPayload,
          binding: None,
          ty,
          init: None,
          memory_decorations: MemoryDecorations::empty(),
        };
        let g_h = self.module.global_variables.append(g, Span::UNDEFINED);
        let g = self.make_expression_inner_raw(naga::Expression::GlobalVariable(g_h), false);

        let return_handle = self.make_new_handle();
        self.expression_mapping.insert(return_handle, g);
        self.global_var_mapping.insert(return_handle, g_h);
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
      PrimitiveShaderValueType::vector(VectorSize::Quad, ScalarType::F32),
      String::from("vertex_point_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::VertexPositionOut),
    )
  }

  fn define_frag_depth_output(&mut self) -> ShaderNodeRawHandle {
    self.define_out(
      PrimitiveShaderValueType::f32(),
      String::from("frag_depth_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::FragDepth),
    )
  }

  fn mark_handle_debug_name(&mut self, h: ShaderNodeRawHandle, name: String) {
    let handle = self.get_expression(h);

    let top_fn = self.building_fn.last_mut().unwrap();
    if let Ok(expr) = top_fn.expressions.try_get(handle) {
      match expr {
        naga::Expression::GlobalVariable(g) => {
          self.module.global_variables.get_mut(*g).name = Some(name.clone());
        }
        naga::Expression::FunctionArgument(idx) => {
          top_fn.arguments[*idx as usize].name = Some(name.clone());
        }
        _ => {
          self
            .building_fn
            .last_mut()
            .unwrap()
            .named_expressions
            .insert(handle, name);
        }
      }
    }
  }

  fn define_const(
    &mut self,
    value: ShaderStructFieldInitValue,
    ty: ShaderSizedValueType,
    inlined: bool,
  ) -> ShaderNodeRawHandle {
    let handle = self.define_const_impl(value, ty, inlined);
    let return_handle = self.make_new_handle();
    self.expression_mapping.insert(return_handle, handle);
    return_handle
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
          let compare = match function {
            AtomicFunction::Exchange { compare, .. } => compare.map(|c| {
              comparison = true;
              self.get_expression(c)
            }),
            _ => None,
          };
          let fun = map_atomic_function(function, compare);

          let primitive = match ty {
            ShaderAtomicValueType::I32 => PrimitiveShaderValueType::i32(),
            ShaderAtomicValueType::U32 => PrimitiveShaderValueType::u32(),
          };

          let ty = if let AtomicFunction::Exchange { weak: true, .. } = function {
            let scalar_ty = map_atomic_scalar(ty);
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
                ShaderBuiltInFunction::Select => {
                  break naga::Expression::Select {
                    condition: self.get_expression(parameters[2]),
                    accept: self.get_expression(parameters[1]),
                    reject: self.get_expression(parameters[0]),
                  };
                }
                ShaderBuiltInFunction::All => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::All,
                    argument: self.get_expression(parameters[0]),
                  };
                }
                ShaderBuiltInFunction::Any => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::Any,
                    argument: self.get_expression(parameters[0]),
                  };
                }
                ShaderBuiltInFunction::IsNan => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::IsNan,
                    argument: self.get_expression(parameters[0]),
                  };
                }
                ShaderBuiltInFunction::IsInf => {
                  break naga::Expression::Relational {
                    fun: naga::RelationalFunction::IsInf,
                    argument: self.get_expression(parameters[0]),
                  };
                }
                ShaderBuiltInFunction::ArrayLength => {
                  break naga::Expression::ArrayLength(self.get_expression(parameters[0]));
                }
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

                  map_math_function(f)
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

                  map_math_function(f)
                }
                f => map_math_function(f),
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
        ShaderNodeExpr::TextureQuery(texture, info) => {
          let level = match info {
            TextureQuery::Size { level } => level.map(|v| self.get_expression(v)),
            _ => None,
          };
          naga::Expression::ImageQuery {
            image: self.get_expression(texture),
            query: map_texture_query(info, level),
          }
        }
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
          gather: gather_channel.map(map_gather_channel),
          coordinate: self.get_expression(position),
          array_index: array_index.map(|index| self.get_expression(index)),
          offset: offset.map(|offset| {
            let data = PrimitiveShaderValue::from(offset);
            self.define_const_impl(
              ShaderStructFieldInitValue::Primitive(data),
              ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vector(
                VectorSize::Bi,
                ScalarType::I32,
              )),
              true,
            )
          }),
          level: map_sample_level(level, |handle| self.get_expression(handle)),
          depth_ref: reference.map(|r| self.get_expression(r)),
          clamp_to_edge: false,
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
          kind: map_scalar_kind(convert_to),
          convert,
        },
        ShaderNodeExpr::Compose { target, parameters } => {
          let mut components: Vec<_> = parameters
            .iter()
            .map(|f| self.get_compose_component(*f))
            .collect();

          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(target.clone())),
            None,
          );
          if let ShaderSizedValueType::Struct(meta) = &target {
            let extra = self.struct_extra_padding_count.get(&meta.name).unwrap();
            for _ in 0..*extra {
              components.push(self.make_expression_inner_raw(
                naga::Expression::Literal(naga::Literal::U32(0)),
                false,
              ));
            }
          }

          naga::Expression::Compose { ty, components }
        }
        ShaderNodeExpr::Derivative { axis, ctrl, source } => naga::Expression::Derivative {
          axis: map_derivative_axis(axis),
          ctrl: map_derivative_control(ctrl),
          expr: self.get_expression(source),
        },
        ShaderNodeExpr::Operator(op) => match op {
          OperatorNode::Unary { one, operator } => naga::Expression::Unary {
            op: map_unary_operator(operator),
            expr: self.get_expression(one),
          },
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
        ShaderNodeExpr::RayQueryProceed { ray_query } => {
          let r = self
            .building_fn
            .last_mut()
            .unwrap()
            .expressions
            .append(naga::Expression::RayQueryProceedResult, Span::UNDEFINED);
          let r_handle = self.make_new_handle();
          self.expression_mapping.insert(r_handle, r);

          self.push_top_statement(naga::Statement::RayQuery {
            query: self.get_expression(ray_query),
            fun: RayQueryFunction::Proceed { result: r },
          });

          return r_handle;
        }
        ShaderNodeExpr::RayQueryGetCandidateIntersection { ray_query } => {
          self.module.generate_ray_intersection_type();
          naga::Expression::RayQueryGetIntersection {
            query: self.get_expression(ray_query),
            committed: false,
          }
        }
        ShaderNodeExpr::RayQueryGetCommittedIntersection { ray_query } => {
          self.module.generate_ray_intersection_type();
          naga::Expression::RayQueryGetIntersection {
            query: self.get_expression(ray_query),
            committed: true,
          }
        }
        ShaderNodeExpr::WorkGroupUniformLoad { pointer, ty } => {
          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(ty)),
            None,
          );
          let r = self.building_fn.last_mut().unwrap().expressions.append(
            naga::Expression::WorkGroupUniformLoadResult { ty },
            Span::UNDEFINED,
          );
          let r_handle = self.make_new_handle();
          self.expression_mapping.insert(r_handle, r);

          self.push_top_statement(naga::Statement::WorkGroupUniformLoad {
            pointer: self.get_expression(pointer),
            result: r,
          });

          return r_handle;
        }
        ShaderNodeExpr::SubgroupBallot { predicate } => {
          let r = self
            .building_fn
            .last_mut()
            .unwrap()
            .expressions
            .append(naga::Expression::SubgroupBallotResult, Span::UNDEFINED);
          let r_handle = self.make_new_handle();
          self.expression_mapping.insert(r_handle, r);

          self.push_top_statement(naga::Statement::SubgroupBallot {
            predicate: Some(self.get_expression(predicate)),
            result: r,
          });

          return r_handle;
        }
        ShaderNodeExpr::SubgroupCollectiveOperation {
          operation,
          collective_operation,
          argument,
          ty,
        } => {
          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(
              ShaderSizedValueType::Primitive(ty),
            )),
            None,
          );
          let r = self.building_fn.last_mut().unwrap().expressions.append(
            naga::Expression::SubgroupOperationResult { ty },
            Span::UNDEFINED,
          );
          let r_handle = self.make_new_handle();
          self.expression_mapping.insert(r_handle, r);

          self.push_top_statement(naga::Statement::SubgroupCollectiveOperation {
            op: map_subgroup_operation(operation),
            collective_op: map_collective_operation(collective_operation),
            argument: self.get_expression(argument),
            result: r,
          });

          return r_handle;
        }
        ShaderNodeExpr::SubgroupGather { mode, argument, ty } => {
          let ty = self.register_ty_impl(
            ShaderValueType::Single(ShaderValueSingleType::Sized(
              ShaderSizedValueType::Primitive(ty),
            )),
            None,
          );
          let r = self.building_fn.last_mut().unwrap().expressions.append(
            naga::Expression::SubgroupOperationResult { ty },
            Span::UNDEFINED,
          );
          let r_handle = self.make_new_handle();
          self.expression_mapping.insert(r_handle, r);

          self.push_top_statement(naga::Statement::SubgroupGather {
            mode: map_subgroup_gather_mode(mode, |handle| self.get_expression(handle)),
            argument: self.get_expression(argument),
            result: r,
          });

          return r_handle;
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

  fn load(&mut self, source: ShaderNodeRawHandle) -> ShaderNodeRawHandle {
    let ex = naga::Expression::Load {
      pointer: self.get_expression(source),
    };
    self.make_expression_inner(ex)
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

  fn ray_query_initialize(
    &mut self,
    query: ShaderNodeRawHandle,
    tlas: BindingNode<ShaderAccelerationStructure>,
    ray_desc: ShaderRayDesc,
  ) {
    let ray_desc_type = self.module.generate_ray_desc_type();

    let ray_desc_raw = self.make_expression_inner(naga::Expression::Compose {
      ty: ray_desc_type,
      components: vec![
        self.get_expression(ray_desc.flags),
        self.get_expression(ray_desc.cull_mask),
        self.get_expression(ray_desc.t_min),
        self.get_expression(ray_desc.t_max),
        self.get_expression(ray_desc.origin),
        self.get_expression(ray_desc.dir),
      ],
    });

    self.push_top_statement(naga::Statement::RayQuery {
      query: self.get_expression(query),
      fun: RayQueryFunction::Initialize {
        acceleration_structure: self.get_expression(tlas.handle()),
        descriptor: self.get_expression(ray_desc_raw),
      },
    });
  }
  fn ray_query_terminate(&mut self, query: ShaderNodeRawHandle) {
    self.push_top_statement(naga::Statement::RayQuery {
      query: self.get_expression(query),
      fun: RayQueryFunction::Terminate,
    });
  }
  // todo ray query confirm hit

  fn push_scope(&mut self) {
    self
      .block
      .push((Default::default(), BlockBuildingState::Common))
  }

  fn pop_scope(&mut self) {
    // pre check module level
    let (_, ty) = self.block.last().unwrap();
    if let BlockBuildingState::Function = ty {
      if self.building_fn.len() == 1
        && let Some(size) = self.output_mesh_task_size
      {
        // task stage must return @builtin(mesh_task_size) vec3<u32> directly,
        // unlike other stages which return a composed output struct
        self.do_return(Some(size));
        let ty = self.register_ty_impl(
          ShaderValueType::Single(ShaderValueSingleType::Sized(
            ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vec3::<u32>()),
          )),
          None,
        );
        let bf = self.building_fn.last_mut().unwrap();
        bf.result = Some(naga::FunctionResult {
          ty,
          binding: Some(naga::Binding::BuiltIn(naga::BuiltIn::MeshTaskSize)),
        });
      } else if self.building_fn.len() == 1 && !self.outputs_define.is_empty() {
        // empty output is possible, for example depth only render target
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
          .map(|local| {
            self.make_expression_inner_raw(naga::Expression::Load { pointer: *local }, false)
          })
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
          let value = map_switch_value(case);
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

    (
      ENTRY_POINT_NAME.to_owned(),
      Box::new(NagaModuleBuildResult {
        log_result: self.log_build_result,
        module: self.module.clone(),
      }),
    )
  }
}

pub struct NagaModuleBuildResult {
  pub log_result: bool,
  pub module: naga::Module,
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
      ShaderFieldDecorator::BuiltIn(bt) => naga::Binding::BuiltIn(map_built_in(bt)),
      ShaderFieldDecorator::Location(location, interpolation) => naga::Binding::Location {
        location: location as u32,
        interpolation: interpolation.map(map_interpolation),
        sampling: None,
        blend_src: None,
        per_primitive: false,
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
    // not using array here because I do not want hit another strange layout issue!
    for i in 0..pad_count {
      let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::u32()),
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
