#![feature(const_type_name)]
#![feature(generic_const_exprs)]

mod abstract_load_store;
mod api_core;
mod binding;
mod compute;
mod graphics;
mod layout;
mod re_export;
mod serialization;
mod u32_load_store;

use std::sync::Arc;
use std::{
  any::{Any, TypeId},
  cell::RefCell,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::*,
};

pub use abstract_load_store::*;
pub use api_core::*;
pub use binding::*;
pub use bytemuck::*;
pub use compute::*;
use fast_hash_collection::*;
pub use graphics::*;
pub use layout::*;
use parking_lot::RwLock;
pub use re_export::*;
pub use rendiation_algebra::*;
pub use rendiation_shader_derives::*;
pub use serialization::*;
pub use u32_load_store::*;

pub type DynamicShaderAPI = Box<dyn ShaderAPI>;

pub enum BarrierScope {
  Storage,
  WorkGroup,
}

/// In current design, the implementation should not panic when the shader is building
/// because the upper layer user may well handled error that not expect panic.
pub trait ShaderAPI {
  fn set_workgroup_size(&mut self, size: (u32, u32, u32));
  fn barrier(&mut self, scope: BarrierScope);

  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle;
  fn define_next_frag_out(&mut self) -> ShaderNodeRawHandle;
  fn define_next_vertex_output(
    &mut self,
    ty: PrimitiveShaderValueType,
    interpolation: Option<ShaderInterpolation>,
  ) -> ShaderNodeRawHandle;
  fn define_vertex_position_output(&mut self) -> ShaderNodeRawHandle;
  fn define_frag_depth_output(&mut self) -> ShaderNodeRawHandle;

  fn make_expression(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle;
  fn make_local_var(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle;
  fn make_zero_val(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle;
  fn store(&mut self, source: ShaderNodeRawHandle, target: ShaderNodeRawHandle);
  fn load(&mut self, source: ShaderNodeRawHandle) -> ShaderNodeRawHandle;
  fn texture_store(&mut self, store: ShaderTextureStore);
  fn ray_query_initialize(
    &mut self,
    tlas: HandleNode<ShaderAccelerationStructure>,
    ray_desc: ShaderRayDesc,
  ) -> ShaderNodeRawHandle;

  fn push_scope(&mut self);
  fn pop_scope(&mut self);
  fn push_if_scope(&mut self, condition: ShaderNodeRawHandle);
  fn push_else_scope(&mut self);
  fn push_loop_scope(&mut self);
  fn do_continue(&mut self);
  fn do_break(&mut self);
  fn begin_switch(&mut self, switch_target: ShaderNodeRawHandle);
  fn push_switch_case_scope(&mut self, case: SwitchCaseCondition);
  fn end_switch(&mut self);

  fn discard(&mut self);

  fn get_fn(&mut self, name: String) -> Option<ShaderUserDefinedFunction>;
  fn begin_define_fn(&mut self, name: String, return_ty: Option<ShaderValueType>);
  fn push_fn_parameter(&mut self, p: ShaderValueType) -> ShaderNodeRawHandle;
  fn do_return(&mut self, v: Option<ShaderNodeRawHandle>);
  fn end_fn_define(&mut self) -> ShaderUserDefinedFunction;

  fn build(&mut self) -> (String, Box<dyn Any>);
}

pub struct ShaderBuildingCtx {
  vertex: DynamicShaderAPI,
  fragment: DynamicShaderAPI,
  compute: DynamicShaderAPI,
  current: Option<ShaderStages>,
}

thread_local! {
  static IN_BUILDING_SHADER_API: RefCell<Option<ShaderBuildingCtx>> = const { RefCell::new(None) };
}

pub(crate) fn call_shader_api<T>(modifier: impl FnOnce(&mut dyn ShaderAPI) -> T) -> T {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| {
    let api = api.as_mut().unwrap();
    let api = match api.current.unwrap() {
      ShaderStages::Vertex => &mut api.vertex,
      ShaderStages::Fragment => &mut api.fragment,
      ShaderStages::Compute => &mut api.compute,
    }
    .as_mut();

    modifier(api)
  })
}

pub(crate) fn set_current_building(current: Option<ShaderStages>) {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| {
    let api = api.as_mut().unwrap();
    api.current = current
  })
}

pub(crate) fn get_current_stage() -> Option<ShaderStages> {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| api.as_mut().unwrap().current)
}

pub fn set_build_api_by(api_builder: &dyn Fn(ShaderStages) -> DynamicShaderAPI) {
  set_build_api(ShaderBuildingCtx {
    vertex: api_builder(ShaderStages::Vertex),
    fragment: api_builder(ShaderStages::Fragment),
    compute: api_builder(ShaderStages::Compute),
    current: None,
  });
}

pub fn set_build_api(a: ShaderBuildingCtx) -> Option<ShaderBuildingCtx> {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| api.replace(a))
}

pub fn take_build_api() -> ShaderBuildingCtx {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| api.take().unwrap())
}
