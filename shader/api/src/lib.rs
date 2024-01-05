#![feature(const_type_name)]

mod api_core;
mod binding;
mod compute;
mod graphics;
mod layout;
mod re_export;
mod type_workaround;

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::*,
};

pub use api_core::*;
pub use binding::*;
pub use bytemuck::*;
pub use compute::*;
use fast_hash_collection::*;
pub use graphics::*;
pub use layout::*;
pub use re_export::*;
pub use rendiation_algebra::*;
pub use rendiation_shader_derives::*;
pub use type_workaround::*;

pub type DynamicShaderAPI = Box<dyn ShaderAPI<Output = Box<dyn Any>>>;

pub enum BarrierScope {
  Storage,
  WorkGroup,
}

pub trait ShaderAPI {
  type Output;

  fn set_workgroup_size(&mut self, size: (u32, u32, u32));
  fn barrier(&mut self, scope: BarrierScope);

  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle;
  fn define_next_frag_out(&mut self) -> ShaderNodeRawHandle;
  fn define_next_vertex_output(&mut self, ty: PrimitiveShaderValueType) -> ShaderNodeRawHandle;
  fn define_vertex_position_output(&mut self) -> ShaderNodeRawHandle;
  fn define_frag_depth_output(&mut self) -> ShaderNodeRawHandle;

  fn make_expression(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle;
  fn make_local_var(&mut self, ty: ShaderValueType) -> ShaderNodeRawHandle;
  fn store(&mut self, source: ShaderNodeRawHandle, target: ShaderNodeRawHandle);
  fn load(&mut self, source: ShaderNodeRawHandle) -> ShaderNodeRawHandle;

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

  fn build(&mut self) -> (String, Self::Output);
}

pub(crate) struct ShaderBuildingCtx {
  vertex: DynamicShaderAPI,
  fragment: DynamicShaderAPI,
  compute: DynamicShaderAPI,
  current: Option<ShaderStages>,
}

thread_local! {
  static IN_BUILDING_SHADER_API: RefCell<Option<ShaderBuildingCtx>> = RefCell::new(None);
}

pub(crate) fn call_shader_api<T>(
  modifier: impl FnOnce(&mut dyn ShaderAPI<Output = Box<dyn Any>>) -> T,
) -> T {
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

pub(crate) fn set_build_api(api_builder: &dyn Fn(ShaderStages) -> DynamicShaderAPI) {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| {
    api.replace(ShaderBuildingCtx {
      vertex: api_builder(ShaderStages::Vertex),
      fragment: api_builder(ShaderStages::Fragment),
      compute: api_builder(ShaderStages::Compute),
      current: None,
    });
  })
}

pub(crate) fn take_build_api() -> ShaderBuildingCtx {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| api.take().unwrap())
}
