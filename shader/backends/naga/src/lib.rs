use core::num::NonZeroU32;
use std::any::Any;

use fast_hash_collection::*;
use naga::{MemoryDecorations, RayQueryFunction, Span};
use rendiation_shader_api::*;

mod conv;
use conv::*;

#[cfg(test)]
mod layout_test;

pub struct ShaderAPINagaImpl {
  module: naga::Module,
  handle_id: usize,
  block: Vec<(Vec<naga::Statement>, BlockBuildingState)>,
  control_structure: Vec<naga::Statement>,
  building_fn: Vec<naga::Function>,
  fn_mapping: FastHashMap<String, naga::Handle<naga::Function>>,
  ty_mapping: FastHashMap<ShaderValueType, naga::Handle<naga::Type>>,
  expression_mapping: FastHashMap<ShaderNodeRawHandle, naga::Handle<naga::Expression>>,
  outputs: Vec<EntryOutput>,
  /// the location of the next user defined output, the builtin outputs do not take the location
  next_output_location: usize,
  /// For the struct that contains explicit padding members, map each member to the field index,
  /// None means it's a padding member.
  padded_structs: FastHashMap<naga::Handle<naga::Type>, Vec<Option<usize>>>,
  layouter: naga::proc::Layouter,
  /// used to resolve the expression type in the building fn, has the same stack as building_fn
  building_fn_typifier: Vec<naga::front::Typifier>,
  log_build_result: bool,
  global_var_mapping: FastHashMap<ShaderNodeRawHandle, naga::Handle<naga::GlobalVariable>>,
  output_mesh_task_size: Option<ShaderNodeRawHandle>,
}

enum BlockBuildingState {
  Common,
  SwitchCase(SwitchCaseCondition),
  Loop,
  IfAccept,
  Else,
  Function,
}

struct EntryOutput {
  meta: ShaderStructFieldMetaInfo,
  /// the local variable that holds the output value, it is loaded and composed into the output
  /// struct when the entry function returns
  var: naga::Handle<naga::Expression>,
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
      outputs: Default::default(),
      next_output_location: 0,
      padded_structs: Default::default(),
      layouter: Default::default(),
      building_fn_typifier: Default::default(),
      log_build_result: false,
      global_var_mapping: Default::default(),
      output_mesh_task_size: Default::default(),
    };

    api.building_fn.push(naga::Function::default());
    api.building_fn_typifier.push(Default::default());
    api
      .block
      .push((Default::default(), BlockBuildingState::Function));

    api
  }

  fn push_top_statement(&mut self, st: naga::Statement) {
    self.block.last_mut().unwrap().0.push(st);
  }

  /// Create a new node that maps to the expression in the building function.
  fn map_new_node(&mut self, expr: naga::Handle<naga::Expression>) -> ShaderNodeRawHandle {
    self.handle_id += 1;
    let node = ShaderNodeRawHandle {
      handle: self.handle_id,
    };
    self.expression_mapping.insert(node, expr);
    node
  }

  fn append_global_expr(&mut self, expr: naga::Expression) -> naga::Handle<naga::Expression> {
    self.module.global_expressions.append(expr, Span::UNDEFINED)
  }

  /// Append the expression into the building function, and emit it if required.
  fn append_fn_expr(&mut self, expr: naga::Expression) -> naga::Handle<naga::Expression> {
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

  fn make_expression_inner(&mut self, expr: naga::Expression) -> ShaderNodeRawHandle {
    let expr = self.append_fn_expr(expr);
    self.map_new_node(expr)
  }

  /// Create the node of the result expression produced by the statement, the result expression
  /// must not be emitted.
  fn make_statement_result(
    &mut self,
    result: naga::Expression,
    statement: impl FnOnce(&mut Self, naga::Handle<naga::Expression>) -> naga::Statement,
  ) -> ShaderNodeRawHandle {
    let result = self
      .building_fn
      .last_mut()
      .unwrap()
      .expressions
      .append(result, Span::UNDEFINED);
    let statement = statement(self, result);
    self.push_top_statement(statement);
    self.map_new_node(result)
  }

  /// The WGSL output refers the mesh output variable and the task payload by name in the entry point
  /// attributes, so they must be named.
  fn name_global_if_unnamed(&mut self, global: naga::Handle<naga::GlobalVariable>, name: &str) {
    let var = self.module.global_variables.get_mut(global);
    if var.name.is_none() {
      var.name = Some(name.to_owned());
    }
  }

  fn declare_global(
    &mut self,
    space: naga::AddressSpace,
    binding: Option<naga::ResourceBinding>,
    ty: naga::Handle<naga::Type>,
  ) -> ShaderNodeRawHandle {
    let global = naga::GlobalVariable {
      name: None,
      space,
      binding,
      ty,
      init: None,
      memory_decorations: MemoryDecorations::empty(),
    };
    let global = self.module.global_variables.append(global, Span::UNDEFINED);
    let node = self.make_expression_inner(naga::Expression::GlobalVariable(global));
    self.global_var_mapping.insert(node, global);
    node
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
      naga::Expression::Literal(_) | naga::Expression::ZeroValue(_) => self.append_fn_expr(expr),
      naga::Expression::Compose { ty, components } => {
        let components = components
          .iter()
          .map(|c| self.copy_global_expr_into_fn(*c))
          .collect();
        self.append_fn_expr(naga::Expression::Compose { ty, components })
      }
      naga::Expression::Constant(c) => self.copy_global_expr_into_fn(self.module.constants[c].init),
      other => unreachable!("unexpected global expression in constant init: {other:?}"),
    }
  }

  fn register_sized_ty(&mut self, ty: ShaderSizedValueType) -> naga::Handle<naga::Type> {
    self.register_ty_impl(ShaderValueType::Single(ShaderValueSingleType::Sized(ty)))
  }

  fn register_primitive_ty(&mut self, ty: PrimitiveShaderValueType) -> naga::Handle<naga::Type> {
    self.register_sized_ty(ShaderSizedValueType::Primitive(ty))
  }

  fn register_ty_impl(&mut self, ty: ShaderValueType) -> naga::Handle<naga::Type> {
    if let Some(handle) = self.ty_mapping.get(&ty) {
      return *handle;
    }

    let mut name = None;
    let mut padded_struct_member_fields = None;

    let naga_ty = match &ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Sized(f) => match f {
          ShaderSizedValueType::Atomic(t) => naga::TypeInner::Atomic(map_atomic_scalar(*t)),
          ShaderSizedValueType::Primitive(p) => map_primitive_type(*p),
          ShaderSizedValueType::Struct(st) => {
            name = st.name.to_owned().into();
            let (inner, member_fields) = gen_struct_define(self, st);
            if member_fields.iter().any(|f| f.is_none()) {
              padded_struct_member_fields = Some(member_fields);
            }
            inner
          }
          ShaderSizedValueType::FixedSizeArray(ty, size) => {
            let base = self.register_sized_ty(*ty.clone());
            naga::TypeInner::Array {
              base,
              size: naga::ArraySize::Constant(NonZeroU32::new(*size as u32).unwrap()),
              stride: self.natural_layout(base).to_stride(),
            }
          }
        },
        ShaderValueSingleType::Unsized(ty) => match ty {
          ShaderUnSizedValueType::UnsizedArray(ty) => {
            let base = self.register_sized_ty(*ty.clone());
            naga::TypeInner::Array {
              base,
              size: naga::ArraySize::Dynamic,
              stride: self.natural_layout(base).to_stride(),
            }
          }
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
        base: self.register_ty_impl(ShaderValueType::Single(ty.clone())),
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
    if let Some(member_fields) = padded_struct_member_fields {
      self.padded_structs.insert(type_handle, member_fields);
    }
    type_handle
  }

  /// The natural WGSL layout of the type, which only depends on the type itself.
  fn natural_layout(&mut self, ty: naga::Handle<naga::Type>) -> naga::proc::TypeLayout {
    self
      .layouter
      .update(self.module.to_ctx())
      .expect("failed to compute the naga type layout");
    self.layouter[ty]
  }

  /// Insert zero values for the padding members if the struct has explicit padding members.
  fn fill_struct_padding_components(
    &mut self,
    ty: naga::Handle<naga::Type>,
    components: Vec<naga::Handle<naga::Expression>>,
    append: fn(&mut Self, naga::Expression) -> naga::Handle<naga::Expression>,
  ) -> Vec<naga::Handle<naga::Expression>> {
    let Some(member_fields) = self.padded_structs.get(&ty).cloned() else {
      return components;
    };
    member_fields
      .iter()
      .map(|field| match field {
        Some(field_index) => components[*field_index],
        None => append(self, naga::Expression::Literal(naga::Literal::U32(0))),
      })
      .collect()
  }

  /// Resolve the type of the expression in the building function.
  fn resolve_expr_type(
    &mut self,
    expr: naga::Handle<naga::Expression>,
  ) -> naga::proc::TypeResolution {
    let function = self.building_fn.last().unwrap();
    let typifier = self.building_fn_typifier.last_mut().unwrap();
    let ctx = naga::proc::ResolveContext::with_locals(
      &self.module,
      &function.local_variables,
      &function.arguments,
    );
    typifier
      .grow(expr, &function.expressions, &ctx)
      .expect("failed to resolve the expression type");
    typifier[expr].clone()
  }

  /// Map the struct field index into the naga struct member index, they are different when the
  /// struct has explicit padding members.
  fn map_struct_field_index(
    &mut self,
    base: naga::Handle<naga::Expression>,
    field_index: usize,
  ) -> u32 {
    if self.padded_structs.is_empty() {
      return field_index as u32;
    }

    let struct_ty = match self.resolve_expr_type(base) {
      naga::proc::TypeResolution::Handle(ty) => match self.module.types[ty].inner {
        naga::TypeInner::Pointer { base, .. } => base,
        _ => ty,
      },
      naga::proc::TypeResolution::Value(naga::TypeInner::Pointer { base, .. }) => base,
      _ => return field_index as u32,
    };

    match self.padded_structs.get(&struct_ty) {
      Some(member_fields) => member_fields
        .iter()
        .position(|f| *f == Some(field_index))
        .expect("struct field index out of bound") as u32,
      None => field_index as u32,
    }
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
    ty: ShaderSizedValueType,
    name: String,
    ty_deco: ShaderFieldDecorator,
  ) -> ShaderNodeRawHandle {
    assert!(self.block.len() == 1); // we should define output in root scope
    assert!(self.building_fn.len() == 1);

    let node = self.make_local_var(ShaderValueType::Single(ShaderValueSingleType::Sized(
      ty.clone(),
    )));
    self.outputs.push(EntryOutput {
      meta: ShaderStructFieldMetaInfo {
        name,
        ty,
        ty_deco: Some(ty_deco),
      },
      var: self.get_expression(node),
    });
    node
  }

  fn define_location_out(
    &mut self,
    ty: ShaderSizedValueType,
    name_prefix: &str,
    interpolation: Option<ShaderInterpolation>,
  ) -> ShaderNodeRawHandle {
    let location = self.next_output_location;
    self.next_output_location += 1;
    self.define_out(
      ty,
      format!("{name_prefix}_{location}"),
      ShaderFieldDecorator::Location(location, interpolation),
    )
  }

  /// Create the expressions of the primitive value in the global expression arena.
  fn global_primitive_expr(
    &mut self,
    data: PrimitiveShaderValue,
  ) -> naga::Handle<naga::Expression> {
    match data {
      PrimitiveShaderValue::Scalar(v) => {
        self.append_global_expr(naga::Expression::Literal(scalar_value_to_naga_literal(v)))
      }
      PrimitiveShaderValue::Vector { size, scalar, data } => {
        self.global_compose_scalars(PrimitiveShaderValueType::vector(size, scalar), data.iter())
      }
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
          .map(|column| self.global_compose_scalars(column_ty, column.iter()))
          .collect();
        self.global_compose(
          PrimitiveShaderValueType::Matrix {
            columns,
            rows,
            scalar,
          },
          components,
        )
      }
    }
  }

  fn global_compose_scalars<'a>(
    &mut self,
    ty: PrimitiveShaderValueType,
    scalars: impl Iterator<Item = &'a ScalarValue>,
  ) -> naga::Handle<naga::Expression> {
    let components = scalars
      .map(|v| self.append_global_expr(naga::Expression::Literal(scalar_value_to_naga_literal(*v))))
      .collect();
    self.global_compose(ty, components)
  }

  fn global_compose(
    &mut self,
    ty: PrimitiveShaderValueType,
    components: Vec<naga::Handle<naga::Expression>>,
  ) -> naga::Handle<naga::Expression> {
    let ty = self.register_primitive_ty(ty);
    self.append_global_expr(naga::Expression::Compose { ty, components })
  }

  fn define_const_global_expr_impl(
    &mut self,
    value: ShaderStructFieldInitValue,
    raw_ty: &ShaderSizedValueType,
  ) -> naga::Handle<naga::Expression> {
    match (value, raw_ty) {
      (ShaderStructFieldInitValue::Primitive(init), ShaderSizedValueType::Primitive(_)) => {
        self.global_primitive_expr(init)
      }
      (ShaderStructFieldInitValue::Struct(init), ShaderSizedValueType::Struct(meta)) => {
        let init: Vec<_> = init
          .iter()
          .zip(meta.fields.iter())
          .map(|(v, f_ty)| self.define_const_global_expr_impl(v.clone(), &f_ty.ty))
          .collect();
        let ty = self.register_sized_ty(raw_ty.clone());
        let components = self.fill_struct_padding_components(ty, init, Self::append_global_expr);
        self.append_global_expr(naga::Expression::Compose { ty, components })
      }
      (ShaderStructFieldInitValue::Array(init), ShaderSizedValueType::FixedSizeArray(f_ty, _)) => {
        let ty = self.register_sized_ty(raw_ty.clone());
        let components = init
          .iter()
          .map(|v| self.define_const_global_expr_impl(v.clone(), f_ty))
          .collect();
        self.append_global_expr(naga::Expression::Compose { ty, components })
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

    let ty = self.register_sized_ty(ty);

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

    self.append_fn_expr(naga::Expression::Constant(constant))
  }

  fn lower_builtin_call(
    &mut self,
    f: ShaderBuiltInFunction,
    parameters: &[ShaderNodeRawHandle],
  ) -> naga::Expression {
    let args: Vec<_> = parameters.iter().map(|p| self.get_expression(*p)).collect();
    let relational = |fun| naga::Expression::Relational {
      fun,
      argument: args[0],
    };
    let math = |fun| naga::Expression::Math {
      fun,
      arg: args[0],
      arg1: args.get(1).copied(),
      arg2: args.get(2).copied(),
      arg3: args.get(3).copied(),
    };

    match f {
      ShaderBuiltInFunction::Select => naga::Expression::Select {
        condition: args[2],
        accept: args[1],
        reject: args[0],
      },
      ShaderBuiltInFunction::All => relational(naga::RelationalFunction::All),
      ShaderBuiltInFunction::Any => relational(naga::RelationalFunction::Any),
      ShaderBuiltInFunction::IsNan => relational(naga::RelationalFunction::IsNan),
      ShaderBuiltInFunction::IsInf => relational(naga::RelationalFunction::IsInf),
      ShaderBuiltInFunction::ArrayLength => naga::Expression::ArrayLength(args[0]),
      ShaderBuiltInFunction::Modf | ShaderBuiltInFunction::Frexp => {
        // the result struct type must be generated before use
        let arg_ty = self.resolve_expr_type(args[0]);
        let (size, scalar) = arg_ty
          .inner_with(&self.module.types)
          .vector_size_and_scalar()
          .expect("modf and frexp require float scalar or vector argument");
        let result_ty = if let ShaderBuiltInFunction::Modf = f {
          naga::PredeclaredType::ModfResult { size, scalar }
        } else {
          naga::PredeclaredType::FrexpResult { size, scalar }
        };
        self.module.generate_predeclared_type(result_ty);
        math(map_math_function(f))
      }
      f => math(map_math_function(f)),
    }
  }
}

impl ShaderAPI for ShaderAPINagaImpl {
  fn log_build_result(&mut self) {
    self.log_build_result = true;
  }

  fn set_workgroup_size(&mut self, size: (u32, u32, u32)) {
    self.module.entry_points[0].workgroup_size = [size.0, size.1, size.2]
  }

  fn set_early_depth_test(&mut self, test: ShaderEarlyDepthTest) {
    self.module.entry_points[0].early_depth_test = Some(map_early_depth_test(test));
  }

  fn barrier(&mut self, scope: BarrierScope) {
    let b = map_barrier(scope);
    self.push_top_statement(naga::Statement::ControlBarrier(b));
  }

  fn define_mesh_info(&mut self, mesh_info: MeshStageInfo) {
    let vertex_output_type = self.register_sized_ty(mesh_info.vertex_output_type);
    let primitive_output_type = self.register_sized_ty(mesh_info.primitive_output_type);

    let output_variable = *self
      .global_var_mapping
      .get(&mesh_info.output_variable)
      .unwrap();
    self.name_global_if_unnamed(output_variable, "mesh_output");

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
    self.name_global_if_unnamed(output_variable, "task_payload");
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
        let data_ty = self.register_primitive_ty(data_ty);
        self.add_fn_input_inner(naga::FunctionArgument {
          name: None,
          ty: data_ty,
          binding: naga::Binding::BuiltIn(map_built_in(ty)).into(),
        })
      }
      ShaderInputNode::Binding {
        desc,
        bindgroup_index,
        entry_index,
      } => {
        let space = map_address_space(desc.get_address_space().unwrap());
        if let ShaderValueType::BindingArray { ty, .. } = &desc.ty
          && space != naga::AddressSpace::Handle
        {
          assert!(
            matches!(
              ty,
              ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(_))
                | ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedStruct(_))
            ),
            "the element of the buffer binding array must be a struct, got: {ty:?}"
          );
        }
        let ty = self.register_ty_impl(desc.ty);
        let binding = naga::ResourceBinding {
          group: bindgroup_index as u32,
          binding: entry_index as u32,
        };
        self.declare_global(space, Some(binding), ty)
      }
      ShaderInputNode::UserDefinedIn {
        ty,
        location,
        interpolation,
      } => {
        let ty = self.register_primitive_ty(ty);
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
        let ty = self.register_sized_ty(ty);
        self.declare_global(naga::AddressSpace::WorkGroup, None, ty)
      }
      ShaderInputNode::Private { ty } => {
        let ty = self.register_sized_ty(ty);
        self.declare_global(naga::AddressSpace::Private, None, ty)
      }
      ShaderInputNode::TaskPayload { ty } => {
        let ty = self.register_sized_ty(ty);
        self.declare_global(naga::AddressSpace::TaskPayload, None, ty)
      }
    }
  }

  fn define_next_frag_out(&mut self, ty: ShaderSizedValueType) -> ShaderNodeRawHandle {
    self.define_location_out(ty, "frag_out", None)
  }

  fn define_next_vertex_output(
    &mut self,
    ty: PrimitiveShaderValueType,
    interpolation: Option<ShaderInterpolation>,
  ) -> ShaderNodeRawHandle {
    self.define_location_out(
      ShaderSizedValueType::Primitive(ty),
      "vertex_out",
      interpolation,
    )
  }

  fn define_vertex_position_output(&mut self, invariant: bool) -> ShaderNodeRawHandle {
    self.define_out(
      ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vec4::<f32>()),
      String::from("vertex_point_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::VertexPositionOut { invariant }),
    )
  }

  fn define_vertex_clip_distances_output(&mut self, count: usize) -> ShaderNodeRawHandle {
    self.define_out(
      ShaderSizedValueType::FixedSizeArray(
        Box::new(ShaderSizedValueType::Primitive(
          PrimitiveShaderValueType::f32(),
        )),
        count,
      ),
      String::from("vertex_clip_distances_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::VertexClipDistances),
    )
  }

  fn define_frag_depth_output(&mut self) -> ShaderNodeRawHandle {
    self.define_out(
      ShaderSizedValueType::Primitive(PrimitiveShaderValueType::f32()),
      String::from("frag_depth_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::FragDepth),
    )
  }

  fn define_frag_sample_mask_output(&mut self) -> ShaderNodeRawHandle {
    self.define_out(
      ShaderSizedValueType::Primitive(PrimitiveShaderValueType::u32()),
      String::from("frag_sample_mask_out"),
      ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::FragSampleMask),
    )
  }

  fn mark_handle_debug_name(&mut self, h: ShaderNodeRawHandle, name: String) {
    let Some(handle) = self.expression_mapping.get(&h) else {
      return;
    };
    let handle = *handle;

    let Some(top_fn) = self.building_fn.last_mut() else {
      return;
    };

    let Ok(expr) = top_fn.expressions.try_get(handle) else {
      return;
    };

    match expr {
      naga::Expression::GlobalVariable(g) => {
        let var = self.module.global_variables.get_mut(*g);
        // avoid override for global var
        if var.name.is_none() {
          var.name = Some(name);
        }
      }
      naga::Expression::FunctionArgument(idx) => {
        top_fn.arguments[*idx as usize].name = Some(name);
      }
      // the local variable expression is never emitted, so the named expression is not used
      naga::Expression::LocalVariable(v) => {
        top_fn.local_variables[*v].name = Some(name);
      }
      _ => {
        top_fn.named_expressions.insert(handle, name);
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
    self.map_new_node(handle)
  }

  fn make_expression(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle {
    let expr = match expr {
      ShaderNodeExpr::Fake => return ShaderNodeRawHandle { handle: 0 },
      ShaderNodeExpr::Zeroed { target } => {
        naga::Expression::ZeroValue(self.register_sized_ty(target))
      }
      ShaderNodeExpr::AtomicCall {
        ty,
        pointer,
        function,
        value,
      } => {
        let compare = match function {
          AtomicFunction::Exchange { compare, .. } => compare.map(|c| self.get_expression(c)),
          _ => None,
        };
        let comparison = compare.is_some();
        let fun = map_atomic_function(function, compare);

        let ty = if let AtomicFunction::Exchange { weak: true, .. } = function {
          self.module.generate_predeclared_type(
            naga::PredeclaredType::AtomicCompareExchangeWeakResult(map_atomic_scalar(ty)),
          )
        } else {
          let primitive = match ty {
            ShaderAtomicValueType::I32 => PrimitiveShaderValueType::i32(),
            ShaderAtomicValueType::U32 => PrimitiveShaderValueType::u32(),
          };
          self.register_primitive_ty(primitive)
        };

        return self.make_statement_result(
          naga::Expression::AtomicResult { ty, comparison },
          |this, result| naga::Statement::Atomic {
            pointer: this.get_expression(pointer),
            fun,
            value: this.get_expression(value),
            result: Some(result),
          },
        );
      }
      ShaderNodeExpr::FunctionCall {
        meta: ShaderFunctionType::Custom(meta),
        parameters,
      } => {
        let function = self.fn_mapping[&meta.name];
        return self.make_statement_result(
          naga::Expression::CallResult(function),
          |this, result| naga::Statement::Call {
            function,
            arguments: parameters.iter().map(|p| this.get_expression(*p)).collect(),
            result: Some(result),
          },
        );
      }
      ShaderNodeExpr::FunctionCall {
        meta: ShaderFunctionType::BuiltIn(f),
        parameters,
      } => self.lower_builtin_call(f, &parameters),
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
        clamp_to_edge,
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
        clamp_to_edge,
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
      ShaderNodeExpr::Swizzle {
        source,
        size,
        pattern,
      } => naga::Expression::Swizzle {
        size: map_vector_size(size),
        vector: self.get_expression(source),
        pattern: pattern.map(|component| match component {
          0 => naga::SwizzleComponent::X,
          1 => naga::SwizzleComponent::Y,
          2 => naga::SwizzleComponent::Z,
          3 => naga::SwizzleComponent::W,
          _ => unreachable!("invalid swizzle component"),
        }),
      },
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
        let components = parameters
          .iter()
          .map(|f| self.get_compose_component(*f))
          .collect();
        let ty = self.register_sized_ty(target);
        let components = self.fill_struct_padding_components(ty, components, Self::append_fn_expr);
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
        } => naga::Expression::Binary {
          op: map_binary_op(operator),
          left: self.get_expression(left),
          right: self.get_expression(right),
        },
        OperatorNode::Index { array, entry } => naga::Expression::Access {
          base: self.get_expression(array),
          index: self.get_expression(entry),
        },
      },
      ShaderNodeExpr::IndexStatic {
        field_index,
        target: struct_node,
      } => {
        let base = self.get_expression(struct_node);
        let index = self.map_struct_field_index(base, field_index);
        naga::Expression::AccessIndex { base, index }
      }
      ShaderNodeExpr::RayQueryProceed { ray_query } => {
        return self.make_statement_result(
          naga::Expression::RayQueryProceedResult,
          |this, result| naga::Statement::RayQuery {
            query: this.get_expression(ray_query),
            fun: RayQueryFunction::Proceed { result },
          },
        );
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
        let ty = self.register_sized_ty(ty);
        return self.make_statement_result(
          naga::Expression::WorkGroupUniformLoadResult { ty },
          |this, result| naga::Statement::WorkGroupUniformLoad {
            pointer: this.get_expression(pointer),
            result,
          },
        );
      }
      ShaderNodeExpr::SubgroupBallot { predicate } => {
        return self.make_statement_result(
          naga::Expression::SubgroupBallotResult,
          |this, result| naga::Statement::SubgroupBallot {
            predicate: Some(this.get_expression(predicate)),
            result,
          },
        );
      }
      ShaderNodeExpr::SubgroupCollectiveOperation {
        operation,
        collective_operation,
        argument,
        ty,
      } => {
        let ty = self.register_primitive_ty(ty);
        return self.make_statement_result(
          naga::Expression::SubgroupOperationResult { ty },
          |this, result| naga::Statement::SubgroupCollectiveOperation {
            op: map_subgroup_operation(operation),
            collective_op: map_collective_operation(collective_operation),
            argument: this.get_expression(argument),
            result,
          },
        );
      }
      ShaderNodeExpr::SubgroupGather { mode, argument, ty } => {
        let ty = self.register_primitive_ty(ty);
        return self.make_statement_result(
          naga::Expression::SubgroupOperationResult { ty },
          |this, result| naga::Statement::SubgroupGather {
            mode: map_subgroup_gather_mode(mode, |handle| this.get_expression(handle)),
            argument: this.get_expression(argument),
            result,
          },
        );
      }
    };

    self.make_expression_inner(expr)
  }

  fn make_zero_val(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
    let ty = self.register_ty_impl(ty);
    self.make_expression_inner(naga::Expression::ZeroValue(ty))
  }

  fn make_local_var(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
    let v = naga::LocalVariable {
      name: None,
      ty: self.register_ty_impl(ty),
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

    let descriptor = self.append_fn_expr(naga::Expression::Compose {
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
        descriptor,
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
        let ty = self.register_primitive_ty(PrimitiveShaderValueType::vec3::<u32>());
        let bf = self.building_fn.last_mut().unwrap();
        bf.result = Some(naga::FunctionResult {
          ty,
          binding: Some(naga::Binding::BuiltIn(naga::BuiltIn::MeshTaskSize)),
        });
      } else if self.building_fn.len() == 1 && !self.outputs.is_empty() {
        // empty output is possible, for example depth only render target
        let ty = ShaderStructMetaInfo {
          name: String::from("ModuleOutput"),
          fields: self.outputs.iter().map(|o| o.meta.clone()).collect(),
          host_layout: None,
        };
        let (ty, _) = gen_struct_define(self, &ty);
        let ty = naga::Type {
          name: None,
          inner: ty,
        };
        let ty = self.module.types.insert(ty, Span::UNDEFINED);

        let output_vars: Vec<_> = self.outputs.iter().map(|o| o.var).collect();
        let components = output_vars
          .into_iter()
          .map(|pointer| self.append_fn_expr(naga::Expression::Load { pointer }))
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
        let mut bf = self.building_fn.pop().unwrap();
        self.building_fn_typifier.pop();
        bf.body = b;
        // is entry
        if self.building_fn.is_empty() {
          self.module.entry_points[0].function = bf;
        } else {
          let name = bf.name.clone().unwrap();
          let handle = self.module.functions.append(bf, Span::UNDEFINED);
          self.fn_mapping.insert(name, handle);
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
    self
      .fn_mapping
      .contains_key(&name)
      .then_some(ShaderUserDefinedFunction { name })
  }

  fn begin_define_fn(&mut self, name: String, return_ty: ShaderValueType) {
    let name = Some(name);
    if self.building_fn.iter().any(|f| f.name.eq(&name)) {
      panic!("recursive fn definition is not allowed")
    }

    assert!(
      !self.fn_mapping.contains_key(name.as_ref().unwrap()),
      "function redefinition"
    );

    let f = naga::Function {
      result: Some(naga::FunctionResult {
        ty: self.register_ty_impl(return_ty),
        binding: None,
      }),
      name,
      ..Default::default()
    };

    self.building_fn.push(f);
    self.building_fn_typifier.push(Default::default());
    self
      .block
      .push((Default::default(), BlockBuildingState::Function));
  }

  fn push_fn_parameter(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle {
    let ty = self.register_ty_impl(ty);
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
    // only the entry function body is left open
    assert!(
      self.building_fn.len() == 1 && self.block.len() == 1 && self.control_structure.is_empty(),
      "the shader scopes are not balanced when building, some scope or function is not closed"
    );
    let entry = &self.module.entry_points[0];
    if matches!(
      entry.stage,
      naga::ShaderStage::Compute | naga::ShaderStage::Task | naga::ShaderStage::Mesh
    ) {
      assert!(
        entry.workgroup_size.iter().all(|v| *v > 0),
        "the workgroup size of the {:?} stage is not configured",
        entry.stage
      );
    }
    self.pop_scope();

    (
      ENTRY_POINT_NAME.to_owned(),
      Box::new(NagaModuleBuildResult {
        log_result: self.log_build_result,
        module: std::mem::take(&mut self.module),
      }),
    )
  }
}

pub struct NagaModuleBuildResult {
  pub log_result: bool,
  pub module: naga::Module,
}

/// The result of building naga struct members, see [build_struct_members]
struct StructMembers {
  members: Vec<naga::StructMember>,
  /// map each member to the field index, None means it's an explicit padding member
  member_fields: Vec<Option<usize>>,
  /// the byte offset right after the last member
  end_offset: u32,
  alignment: naga::proc::Alignment,
}

impl StructMembers {
  /// Append u32 padding members until the end offset reaches the target offset.
  fn pad_to(&mut self, api: &mut ShaderAPINagaImpl, target: u32) {
    assert!(target >= self.end_offset);
    // all shader types' size are multiple of 4 bytes, so the gap is always able to be filled
    assert!((target - self.end_offset).is_multiple_of(4));
    let u32_ty = api.register_primitive_ty(PrimitiveShaderValueType::u32());
    while self.end_offset < target {
      let padding_index = self.member_fields.iter().filter(|f| f.is_none()).count();
      self.members.push(naga::StructMember {
        name: format!("padding_{padding_index}").into(),
        ty: u32_ty,
        binding: None,
        offset: self.end_offset,
      });
      self.member_fields.push(None);
      self.end_offset += 4;
    }
  }
}

/// Build the struct members in the natural WGSL layout, which means the layout is fully
/// determined by the member types, without any explicit offset or size attribute.
///
/// This is required because some backends ignore the member offsets and span in naga IR and
/// recompute the layout from the member types, for example the WGSL text output (used by the
/// browser WebGPU implementation) and the GLSL output. Any layout that is not natural will
/// silently change in these backends.
///
/// For host shareable struct, the host layout is the source of truth. If the host offset is
/// larger than the natural one (for example std140 requires the nested struct aligned to 16),
/// explicit u32 padding members are inserted to make it natural.
fn build_struct_members(
  api: &mut ShaderAPINagaImpl,
  struct_name: &str,
  fields: &[ShaderStructFieldMetaInfo],
  host_layout: Option<&ShaderStructHostLayout>,
) -> StructMembers {
  let mut result = StructMembers {
    members: Vec::with_capacity(fields.len()),
    member_fields: Vec::with_capacity(fields.len()),
    end_offset: 0,
    alignment: naga::proc::Alignment::ONE,
  };

  for (index, field) in fields.iter().enumerate() {
    let ty = api.register_sized_ty(field.ty.clone());
    let layout = api.natural_layout(ty);
    result.alignment = result.alignment.max(layout.alignment);
    let natural_offset = layout.alignment.round_up(result.end_offset);

    let offset = if let Some(host_layout) = host_layout {
      let host_offset = host_layout.field_offsets[index] as u32;
      assert!(
        host_offset >= natural_offset && layout.alignment.is_aligned(host_offset),
        "the {:?} host layout of struct `{struct_name}` field `{}` is invalid for WGSL, \
         host offset: {host_offset}, natural offset: {natural_offset}",
        host_layout.target,
        field.name,
      );
      if host_offset != natural_offset {
        result.pad_to(api, host_offset);
      }
      host_offset
    } else {
      natural_offset
    };

    let binding = field.ty_deco.map(|deco| match deco {
      ShaderFieldDecorator::BuiltIn(bt) => naga::Binding::BuiltIn(map_built_in(bt)),
      ShaderFieldDecorator::Location(location, interpolation) => naga::Binding::Location {
        location: location as u32,
        interpolation: interpolation.map(map_interpolation),
        sampling: None,
        blend_src: None,
        per_primitive: false,
      },
    });

    result.members.push(naga::StructMember {
      name: field.name.clone().into(),
      ty,
      binding,
      offset,
    });
    result.member_fields.push(Some(index));
    result.end_offset = offset + layout.size;
  }

  result
}

/// return the struct type and the member to field index mapping
fn gen_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: &ShaderStructMetaInfo,
) -> (naga::TypeInner, Vec<Option<usize>>) {
  let mut members = build_struct_members(api, &meta.name, &meta.fields, meta.host_layout.as_ref());
  assert!(!members.members.is_empty());

  let natural_span = members.alignment.round_up(members.end_offset);
  let span = if let Some(host_layout) = &meta.host_layout {
    let host_size = host_layout.size as u32;
    assert!(
      host_size >= natural_span && members.alignment.is_aligned(host_size),
      "the {:?} host layout of struct `{}` is invalid for WGSL, \
       host size: {host_size}, natural size: {natural_span}",
      host_layout.target,
      meta.name,
    );
    members.pad_to(api, host_size);
    host_size
  } else {
    natural_span
  };

  let inner = naga::TypeInner::Struct {
    members: members.members,
    span,
  };
  (inner, members.member_fields)
}

fn gen_unsized_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: &ShaderUnSizedStructMetaInfo,
) -> naga::TypeInner {
  let mut members = build_struct_members(api, &meta.name, &meta.sized_fields, None);

  let (name, array_ty) = &meta.last_dynamic_array_field;
  let ty = api.register_ty_impl(ShaderValueType::Single(ShaderValueSingleType::Unsized(
    ShaderUnSizedValueType::UnsizedArray(Box::new(*array_ty.clone())),
  )));
  // the size of runtime sized array is treated as its stride
  let layout = api.natural_layout(ty);
  let offset = layout.alignment.round_up(members.end_offset);
  let alignment = members.alignment.max(layout.alignment);

  members.members.push(naga::StructMember {
    name: name.to_string().into(),
    ty,
    binding: None,
    offset,
  });

  naga::TypeInner::Struct {
    members: members.members,
    span: alignment.round_up(offset + layout.size),
  }
}
