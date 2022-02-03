use arena_graph::*;

pub use shader_derives::*;

use std::{any::Any, cell::Cell};

pub mod code_gen;
pub use code_gen::*;

pub mod control;
pub mod graph;
pub mod meta;
pub mod nodes;
pub mod operator;
pub mod provider;
pub mod shader_builder;
pub mod structor;
pub mod swizzle;
pub mod traits_impl;
pub mod types;
pub use control::*;
pub use graph::*;
pub use meta::*;
pub use nodes::*;
pub use provider::*;
pub use shader_builder::*;
pub use structor::*;
pub use traits_impl::*;
pub use types::*;

use rendiation_algebra::*;

#[cfg(test)]
mod test;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderStages {
  Vertex,
  Fragment,
}

#[derive(Clone, Copy)]
pub struct ShaderTexture;
#[derive(Clone, Copy)]
pub struct ShaderSampler;

#[derive(Clone)]
pub struct Node<T> {
  pub handle: Cell<ShaderGraphNodeRawHandle<T>>,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderGraphNodeRawHandle<T> {
    self.handle.get()
  }
}

impl<T: ShaderGraphNodeType> From<ShaderGraphNodeRawHandle<T>> for Node<T> {
  fn from(handle: ShaderGraphNodeRawHandle<T>) -> Self {
    Node {
      handle: Cell::new(handle),
    }
  }
}

pub type NodeUntyped = Node<AnyType>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub struct ShaderGraphNodeRawHandle<T> {
  handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>,
  graph_id: usize,
}

impl<T> ShaderGraphNodeRawHandle<T> {
  /// # Safety
  ///
  /// force type casting
  pub unsafe fn cast_type<X>(&self) -> ShaderGraphNodeRawHandle<X> {
    let t: &ShaderGraphNodeRawHandle<X> = std::mem::transmute(self);
    *t
  }

  pub fn cast_untyped(&self) -> ShaderGraphNodeRawHandleUntyped {
    unsafe { self.cast_type() }
  }
}

impl<T> Clone for ShaderGraphNodeRawHandle<T> {
  fn clone(&self) -> ShaderGraphNodeRawHandle<T> {
    Self {
      handle: self.handle,
      graph_id: self.graph_id,
    }
  }
}

impl<T> Copy for ShaderGraphNodeRawHandle<T> {}

impl<T> PartialEq for ShaderGraphNodeRawHandle<T> {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle && self.graph_id == other.graph_id
  }
}

impl<T> Eq for ShaderGraphNodeRawHandle<T> {}

use core::hash::Hash;
impl<T> Hash for ShaderGraphNodeRawHandle<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.handle.hash(state);
  }
}

pub type ShaderGraphNodeRawHandleUntyped = ShaderGraphNodeRawHandle<AnyType>;
