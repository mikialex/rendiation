use arena_graph::*;

use lazy_static::lazy_static;
use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

pub mod builder;
mod code_gen;
pub mod nodes;
pub mod provider;
pub mod shader_function;
pub mod webgpu;
pub use builder::*;
pub use nodes::*;
pub use provider::*;
pub use webgpu::*;
use rendiation_math::*;
pub use shader_function::*;

lazy_static! {
  pub static ref IN_BUILDING_SHADER_GRAPH: Mutex<Option<ShaderGraph>> = Mutex::new(None);
}

pub type ShaderGraphNodeHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;
pub type ShaderGraphNodeHandleUntyped = ShaderGraphNodeHandle<AnyType>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub enum ShaderGraphUniformInputType {
  NoneUBO(ShaderGraphNodeHandleUntyped),
  UBO((Arc<UBOInfo>, Vec<ShaderGraphNodeHandleUntyped>)),
}

pub struct ShaderGraph {
  pub attributes: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,
  pub vertex_position: Option<ShaderGraphNodeHandle<Vec4<f32>>>,

  pub varyings: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,
  pub frag_outputs: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,

  pub bindgroups: Vec<ShaderGraphBindGroup>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,

  pub type_id_map: HashMap<TypeId, &'static str>, // totally hack
}

impl ShaderGraph {
  fn new() -> Self {
    Self {
      attributes: HashSet::new(),
      bindgroups: Vec::new(),
      nodes: ArenaGraph::new(),
      vertex_position: None,
      varyings: HashSet::new(),
      frag_outputs: HashSet::new(),
      type_id_map: HashMap::new(),
    }
  }

  fn register_type<T: ShaderGraphNodeType>(&mut self) {
    self
      .type_id_map
      .entry(TypeId::of::<T>())
      .or_insert_with(|| T::to_glsl_type());
  }
}

pub fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraph) -> T) -> T {
  let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
  let graph = guard.as_mut().unwrap();
  modifier(graph)
}

pub struct ShaderGraphBindGroup {
  pub inputs: Vec<ShaderGraphUniformInputType>,
}
