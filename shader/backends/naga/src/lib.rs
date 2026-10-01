use std::any::Any;

use fast_hash_collection::*;
use naga::{RayQueryFunction, Span};
use rendiation_shader_api::*;

mod constant;
mod conv;
mod entry;
mod expr;
mod function;
mod node;
mod ty;

use conv::*;
use entry::*;
use function::*;
use node::*;
use ty::*;

#[cfg(test)]
mod layout_test;

pub struct ShaderAPINagaImpl {
  module: naga::Module,
  /// indexed by [ShaderNodeRawHandle::handle], the handles are sequential and the handle 0 is
  /// the fake node
  nodes: Vec<NodeSlot>,
  /// The functions being built, the first one is the entry function, the last one is the
  /// building function. A function can be defined while building another one.
  functions: Vec<FunctionBuilder>,
  /// the id of the next defined function, see [FunctionBuilder::id]
  next_fn_id: u32,
  entry: EntryState,
  fn_mapping: FastHashMap<String, naga::Handle<naga::Function>>,
  ty_mapping: FastHashMap<ShaderValueType, naga::Handle<naga::Type>>,
  /// For the struct that contains explicit padding members, map each member to the field index,
  /// None means it's a padding member.
  padded_structs: FastHashMap<naga::Handle<naga::Type>, Vec<Option<usize>>>,
  layouter: naga::proc::Layouter,
  log_build_result: bool,
}

impl ShaderAPINagaImpl {
  pub fn new(stage: ShaderStage) -> Self {
    Self {
      module: Default::default(),
      nodes: vec![NodeSlot::Fake],
      functions: vec![FunctionBuilder::new(0, Default::default())],
      next_fn_id: 1,
      entry: EntryState::new(map_stage(stage)),
      fn_mapping: Default::default(),
      ty_mapping: Default::default(),
      padded_structs: Default::default(),
      layouter: Default::default(),
      log_build_result: false,
    }
  }
}

impl ShaderAPI for ShaderAPINagaImpl {
  fn log_build_result(&mut self) {
    self.log_build_result = true;
  }

  fn set_workgroup_size(&mut self, size: (u32, u32, u32)) {
    self.entry.workgroup_size = [size.0, size.1, size.2]
  }

  fn set_early_depth_test(&mut self, test: ShaderEarlyDepthTest) {
    self.entry.early_depth_test = Some(map_early_depth_test(test));
  }

  fn barrier(&mut self, scope: BarrierScope) {
    let b = map_barrier(scope);
    self.push_top_statement(naga::Statement::ControlBarrier(b));
  }

  fn define_mesh_info(&mut self, mesh_info: MeshStageInfo) {
    let vertex_output_type = self.register_sized_ty(mesh_info.vertex_output_type);
    let primitive_output_type = self.register_sized_ty(mesh_info.primitive_output_type);

    let output_variable = self.get_global(mesh_info.output_variable);
    self.name_global_if_unnamed(output_variable, "mesh_output");

    self.entry.mesh_info = Some(naga::MeshStageInfo {
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
    let output_variable = self.get_global(payload);
    self.name_global_if_unnamed(output_variable, "task_payload");
    self.entry.task_payload = Some(output_variable);
  }

  fn set_output_mesh_task_size(&mut self, size: ShaderNodeRawHandle) {
    self.entry.mesh_task_size = Some(size);
  }

  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle {
    self.define_module_input_impl(input)
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
    self.mark_handle_debug_name_impl(h, name)
  }

  fn define_const(
    &mut self,
    value: ShaderStructFieldInitValue,
    ty: ShaderSizedValueType,
    inlined: bool,
  ) -> ShaderNodeRawHandle {
    let constant = self.define_const_impl(value, ty, inlined);
    self.new_node(NodeSlot::Constant(constant))
  }

  fn make_expression(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle {
    self.make_expression_impl(expr)
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
      .building_fn_mut()
      .function
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

    let components = [
      ray_desc.flags,
      ray_desc.cull_mask,
      ray_desc.t_min,
      ray_desc.t_max,
      ray_desc.origin,
      ray_desc.dir,
    ]
    .into_iter()
    .map(|v| self.get_expression(v))
    .collect();
    let descriptor = self.append_fn_expr(naga::Expression::Compose {
      ty: ray_desc_type,
      components,
    });

    let query = self.get_expression(query);
    let acceleration_structure = self.get_expression(tlas.handle());
    self.push_top_statement(naga::Statement::RayQuery {
      query,
      fun: RayQueryFunction::Initialize {
        acceleration_structure,
        descriptor,
      },
    });
  }
  fn ray_query_terminate(&mut self, query: ShaderNodeRawHandle) {
    let query = self.get_expression(query);
    self.push_top_statement(naga::Statement::RayQuery {
      query,
      fun: RayQueryFunction::Terminate,
    });
  }
  // todo ray query confirm hit

  fn pop_scope(&mut self) {
    let frame = self.building_fn_mut().frames.pop().unwrap();
    let block = naga::Block::from_vec(frame.statements);
    let statement = match frame.kind {
      FrameKind::IfAccept { condition } => naga::Statement::If {
        condition,
        accept: block,
        reject: Default::default(),
      },
      FrameKind::Else { condition, accept } => naga::Statement::If {
        condition,
        accept,
        reject: block,
      },
      FrameKind::Loop => naga::Statement::Loop {
        body: block,
        continuing: Default::default(),
        break_if: None,
      },
      FrameKind::SwitchCase(value) => {
        let FrameKind::Switch { cases, .. } = &mut self.building_fn_mut().top_frame_mut().kind
        else {
          panic!("expect switch")
        };
        cases.push(naga::SwitchCase {
          value,
          body: block,
          fall_through: false,
        });
        return;
      }
      FrameKind::Body => panic!(
        "the shader scopes are not balanced, pop_scope can not close the function body, \
         it is closed by end_fn_define or build"
      ),
      FrameKind::Switch { .. } => panic!(
        "the shader scopes are not balanced, pop_scope can not close the switch, \
         it is closed by end_switch"
      ),
    };
    self.push_top_statement(statement);
  }

  fn push_if_scope(&mut self, condition: ShaderNodeRawHandle) {
    let condition = self.get_expression(condition);
    self
      .building_fn_mut()
      .push_frame(FrameKind::IfAccept { condition });
  }

  fn push_else_scope(&mut self) {
    // find last if block in the top level statements
    let top_statements = self.building_fn_mut().statements_mut();
    let index = top_statements
      .iter()
      .rposition(|s| matches!(s, naga::Statement::If { .. }))
      .expect("expect if clause");
    // the else block replaces the reject of the if
    let naga::Statement::If {
      condition, accept, ..
    } = top_statements.remove(index)
    else {
      unreachable!()
    };

    self
      .building_fn_mut()
      .push_frame(FrameKind::Else { condition, accept });
  }

  fn push_loop_scope(&mut self) {
    self.building_fn_mut().push_frame(FrameKind::Loop);
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
    self.building_fn_mut().push_frame(FrameKind::Switch {
      selector,
      cases: Default::default(),
    });
  }

  fn push_switch_case_scope(&mut self, case: SwitchCaseCondition) {
    let value = map_switch_value(case);
    self
      .building_fn_mut()
      .push_frame(FrameKind::SwitchCase(value));
  }

  fn end_switch(&mut self) {
    let frame = self.building_fn_mut().frames.pop().unwrap();
    let FrameKind::Switch { selector, cases } = frame.kind else {
      panic!("the shader scopes are not balanced, end_switch closes a scope that is not a switch")
    };
    self.push_top_statement(naga::Statement::Switch { selector, cases });
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
    if self.functions.iter().any(|f| f.function.name.eq(&name)) {
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

    let id = self.next_fn_id;
    self.next_fn_id += 1;
    self.functions.push(FunctionBuilder::new(id, f));
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
    // the entry function is closed by build
    assert!(
      self.functions.len() > 1,
      "end_fn_define is called without begin_define_fn"
    );
    let f = self.functions.pop().unwrap().finish();
    let name = f.name.clone().unwrap();
    let handle = self.module.functions.append(f, Span::UNDEFINED);
    self.fn_mapping.insert(name.clone(), handle);
    ShaderUserDefinedFunction { name }
  }

  fn build(&mut self) -> (String, Box<dyn Any>) {
    self.finish_entry_point();
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
