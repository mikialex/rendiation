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
use rendiation_math::*;
use rendiation_ral::ShaderStage;
use rendiation_webgpu::{load_glsl, PipelineBuilder, WGPURenderer};
pub use shader_function::*;
pub use webgpu::*;

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
  pub attributes: Vec<(ShaderGraphNodeHandleUntyped, usize)>,
  pub vertex_position: Option<ShaderGraphNodeHandle<Vec4<f32>>>,

  pub varyings: Vec<(ShaderGraphNodeHandleUntyped, usize)>,
  pub frag_outputs: Vec<(ShaderGraphNodeHandleUntyped, usize)>,

  pub bindgroups: Vec<ShaderGraphBindGroup>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,

  pub type_id_map: HashMap<TypeId, &'static str>, // totally hack
}

impl ShaderGraph {
  fn new() -> Self {
    Self {
      attributes: Vec::new(),
      bindgroups: Vec::new(),
      nodes: ArenaGraph::new(),
      vertex_position: None,
      varyings: Vec::new(),
      frag_outputs: Vec::new(),
      type_id_map: HashMap::new(),
    }
  }

  pub fn create_pipeline<'a>(&self, renderer: &'a WGPURenderer) -> PipelineBuilder<'a> {
    PipelineBuilder::new(
      renderer,
      load_glsl(self.gen_code_vertex(), rendiation_ral::ShaderStage::Vertex),
      load_glsl(self.gen_code_frag(), rendiation_ral::ShaderStage::Fragment),
    )
  }

  pub fn register_type<T: ShaderGraphNodeType>(&mut self) {
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
  pub inputs: Vec<(ShaderGraphUniformInputType, ShaderStage)>,
}
