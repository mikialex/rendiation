use arena_graph::*;
use std::marker::PhantomData;

pub struct ShaderGraph {
  nodes: ArenaGraph<ShaderGraphNode<AnyType>>
}

pub struct ShaderGraphNode<T>{
  phantom: PhantomData<T>
}

pub struct AnyType{}

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}

struct ToneMapping {
  value: f32,
}
