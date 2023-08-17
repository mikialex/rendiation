#![feature(const_type_name)]
#![feature(adt_const_params)]
#![feature(associated_const_equality)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![feature(local_key_cell_methods)]

mod api_core;
mod compute;
mod graphics;
mod layout;
mod re_export;

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::*,
};

pub use api_core::*;
pub use bytemuck::*;
pub use compute::*;
use fast_hash_collection::*;
pub use graphics::*;
pub use layout::*;
pub use re_export::*;
pub use rendiation_algebra::*;
pub use rendiation_shader_derives::*;

pub type DynamicShaderAPI = Box<dyn ShaderAPI<Output = Box<dyn Any>>>;

pub trait ShaderAPI {
  type Output;
  fn define_module_input(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle;
  fn define_frag_out(&mut self) -> ShaderNodeRawHandle;
  fn define_vertex_output(&mut self, ty: PrimitiveShaderValueType) -> ShaderNodeRawHandle;
  fn define_vertex_position_output(&mut self) -> ShaderNodeRawHandle;

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

pub trait TruthCheckPass {}

pub struct TruthCheckBool<const TERM: bool>();

impl TruthCheckPass for TruthCheckBool<true> {}
