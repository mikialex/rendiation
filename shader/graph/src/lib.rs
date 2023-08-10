#![feature(const_type_name)]
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
pub use shader_derives::*;

pub trait ShaderAPI {
  fn register_ty(&mut self, ty: ShaderValueType);

  fn define_module_input(&mut self, input: ShaderGraphInputNode) -> ShaderGraphNodeRawHandle;
  fn define_frag_out(&mut self, idx: usize) -> ShaderGraphNodeRawHandle;

  fn make_expression(&mut self, expr: ShaderGraphNodeExpr) -> ShaderGraphNodeRawHandle;
  fn make_var(&mut self, ty: ShaderValueType) -> ShaderGraphNodeRawHandle;
  fn write(&mut self, source: ShaderGraphNodeRawHandle, target: ShaderGraphNodeRawHandle);
  fn load(&mut self, source: ShaderGraphNodeRawHandle) -> ShaderGraphNodeRawHandle;

  fn push_scope(&mut self);
  fn pop_scope(&mut self);
  fn push_if_scope(&mut self, condition: ShaderGraphNodeRawHandle);
  fn discard(&mut self);
  fn push_for_scope(&mut self, target: ShaderIterator) -> ForNodes;
  fn do_continue(&mut self, looper: ShaderGraphNodeRawHandle);
  fn do_break(&mut self, looper: ShaderGraphNodeRawHandle);

  fn get_fn(&mut self, name: String) -> Option<ShaderFunctionMetaInfo>;
  fn begin_define_fn(&mut self, name: String, return_ty: Option<ShaderValueType>);
  fn push_fn_parameter(&mut self, p: ShaderValueType) -> ShaderGraphNodeRawHandle;
  fn do_return(&mut self, v: Option<ShaderGraphNodeRawHandle>);
  fn end_fn_define(&mut self) -> ShaderFunctionMetaInfo;

  fn build(&mut self) -> (String, String);
}
