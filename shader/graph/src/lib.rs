use arena_graph::*;

pub use shader_derives::*;

use std::{any::Any, cell::Cell};

mod code_gen;

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
pub use control::*;
pub use graph::*;
pub use meta::*;
pub use nodes::*;
pub use provider::*;
pub use shader_builder::*;
pub use structor::*;
pub use traits_impl::*;

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
  pub handle: Cell<ArenaGraphNodeHandle<ShaderGraphNode<T>>>,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ArenaGraphNodeHandle<ShaderGraphNode<T>> {
    self.handle.get()
  }
}

impl<T: ShaderGraphNodeType> From<ArenaGraphNodeHandle<ShaderGraphNode<T>>> for Node<T> {
  fn from(handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>) -> Self {
    Node {
      handle: Cell::new(handle),
    }
  }
}

pub type NodeUntyped = Node<AnyType>;
pub type ShaderGraphNodeRawHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;
pub type ShaderGraphNodeRawHandleUntyped = ArenaGraphNodeHandle<ShaderGraphNode<AnyType>>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub trait SemanticShaderValue: Any {
  type ValueType;
  const NAME: &'static str = "unnamed";
  const STAGE: ShaderStages;
}

pub enum ShaderGraphBuildError {
  MissingRequiredDependency,
}

pub trait ShaderGraphProvider {
  fn build(&self) -> Result<(), ShaderGraphBuildError>;
}
