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
pub mod operator;
pub mod provider;
pub mod shader_function;
pub mod swizzle;
pub mod traits_impl;
pub mod webgpu;
pub use builder::*;
pub use nodes::*;
pub use provider::*;
use rendiation_math::*;
use rendiation_ral::ShaderStage;
use rendiation_webgpu::{load_glsl, PipelineShaderInterfaceInfo, WGPUPipeline};
pub use shader_function::*;
pub use traits_impl::*;
pub use webgpu::*;

lazy_static! {
  pub static ref IN_BUILDING_SHADER_GRAPH: Mutex<Option<ShaderGraph>> = Mutex::new(None);
}

#[derive(Copy, Clone)]
pub struct ShaderGraphNodeHandle<T: ShaderGraphNodeType> {
  pub handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>,
}

impl<T: ShaderGraphNodeType> From<ArenaGraphNodeHandle<ShaderGraphNode<T>>>
  for ShaderGraphNodeHandle<T>
{
  fn from(handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>) -> Self {
    ShaderGraphNodeHandle { handle }
  }
}

pub type ShaderGraphNodeHandleUntyped = ShaderGraphNodeHandle<AnyType>;
pub type ShaderGraphNodeRawHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;
pub type ShaderGraphNodeRawHandleUntyped = ArenaGraphNodeHandle<ShaderGraphNode<AnyType>>;
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

  wgpu_shader_interface: PipelineShaderInterfaceInfo,
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
      wgpu_shader_interface: PipelineShaderInterfaceInfo::new(),
    }
  }

  pub fn create_pipeline(&self) -> WGPUPipeline {
    WGPUPipeline::new(
      load_glsl(self.gen_code_vertex(), rendiation_ral::ShaderStage::Vertex),
      load_glsl(self.gen_code_frag(), rendiation_ral::ShaderStage::Fragment),
      self.wgpu_shader_interface.clone(),
    )
  }

  pub fn insert_node<T: ShaderGraphNodeType>(
    &mut self,
    node: ShaderGraphNode<T>,
  ) -> ShaderGraphNodeHandleUntyped {
    self.register_type::<T>();
    self.nodes.create_node(node.to_any()).into()
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
