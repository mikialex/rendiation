use arena_graph::*;
use std::marker::PhantomData;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
  // we should remove mutex and use unsafe in future
  pub static ref IN_BUILDING_SHADER_GRAPH: Mutex<Option<ShaderGraph>> = Mutex::new(None);
}

pub type ShaderGraphNodeHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;

pub struct ShaderGraph {
  pub nodes: ArenaGraph<ShaderGraphNode<AnyType>>,
}

pub struct ShaderGraphBuilder{}
 
impl ShaderGraphBuilder{
  pub fn new() -> Self{
    // check sketenton or panic
    Self{}
  }

  pub fn create() -> ShaderGraph{
    todo!()
  }
}

pub struct AnyType {}

pub struct ShaderGraphNode<T> {
  phantom: PhantomData<T>,
  data: ShaderGraphNodeData,
}

impl<T> ShaderGraphNode<T> {
  pub fn new(data: ShaderGraphNodeData) -> Self {
    Self {
      data,
      phantom: PhantomData,
    }
  }
}

pub enum ShaderGraphNodeData {
  Function(FunctionNode),
}

pub struct FunctionNode {
  pub function_name: &'static str,
  pub function_source: &'static str,
}

pub struct UniformNode {}

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}

pub trait StaticShaderFunction {
  fn name() -> &'static str;
  fn source() -> &'static str;
}
