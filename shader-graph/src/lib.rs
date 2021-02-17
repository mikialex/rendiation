use arena_graph::*;

use lazy_static::lazy_static;
use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
  sync::Mutex,
};

mod code_gen;

pub mod builder;
pub mod meta;
pub mod nodes;
pub mod operator;
pub mod provider;
pub mod swizzle;
pub mod traits_impl;
pub use builder::*;
pub use meta::*;
pub use nodes::*;
pub use provider::*;
pub use traits_impl::*;

use rendiation_algebra::*;
use rendiation_ral::{PipelineShaderInterfaceInfo, ShaderStage};

#[derive(Copy, Clone)]
pub struct Node<T: ShaderGraphNodeType> {
  pub handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>,
}

impl<T: ShaderGraphNodeType> From<ArenaGraphNodeHandle<ShaderGraphNode<T>>> for Node<T> {
  fn from(handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>) -> Self {
    Node { handle }
  }
}

pub type NodeUntyped = Node<AnyType>;
pub type ShaderGraphNodeRawHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;
pub type ShaderGraphNodeRawHandleUntyped = ArenaGraphNodeHandle<ShaderGraphNode<AnyType>>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub enum ShaderGraphUniformInputType {
  NoneUBO(NodeUntyped),
  UBO((&'static UBOMetaInfo, Vec<NodeUntyped>)),
}

pub struct ShaderGraphCompileResult {
  pub vertex_shader: String,
  pub frag_shader: String,
  pub shader_interface_info: PipelineShaderInterfaceInfo,
}

pub struct ShaderGraph {
  pub attributes: Vec<(NodeUntyped, usize)>,
  pub vertex_position: Option<Node<Vec4<f32>>>,

  pub varyings: Vec<(NodeUntyped, usize)>,
  pub frag_outputs: Vec<(NodeUntyped, usize)>,

  pub bindgroups: Vec<ShaderGraphBindGroup>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,

  pub type_id_map: HashMap<TypeId, &'static str>, // totally hack

  pub shader_interface: PipelineShaderInterfaceInfo,
}

pub struct ShaderGraphBindGroup {
  pub inputs: Vec<(ShaderGraphUniformInputType, ShaderStage)>,
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
      shader_interface: PipelineShaderInterfaceInfo::new(),
    }
  }

  pub fn compile(&self) -> ShaderGraphCompileResult {
    // do extra naga check;
    let vertex = self.gen_code_vertex();
    let frag = self.gen_code_frag();

    let naga_vertex_ir = naga::front::glsl::parse_str(
      &vertex,
      "main",
      naga::ShaderStage::Vertex,
      HashMap::default(),
    );
    let naga_frag_ir = naga::front::glsl::parse_str(
      &frag,
      "main",
      naga::ShaderStage::Fragment,
      HashMap::default(),
    );
    if naga_vertex_ir.is_err() {
      println!("{:?}", naga_vertex_ir);
      println!("{:}", vertex);
    }
    if naga_frag_ir.is_err() {
      println!("{:?}", naga_frag_ir);
      println!("{:}", frag);
    }
    ShaderGraphCompileResult {
      vertex_shader: vertex,
      frag_shader: frag,
      shader_interface_info: self.shader_interface.clone(),
    }
  }

  pub fn insert_node<T: ShaderGraphNodeType>(&mut self, node: ShaderGraphNode<T>) -> NodeUntyped {
    self.register_type::<T>();
    self.nodes.create_node(node.into_any()).into()
  }

  pub fn register_type<T: ShaderGraphNodeType>(&mut self) {
    self
      .type_id_map
      .entry(TypeId::of::<T>())
      .or_insert_with(T::to_glsl_type);
  }
}

lazy_static! {
  pub static ref IN_BUILDING_SHADER_GRAPH: Mutex<Option<ShaderGraph>> = Mutex::new(None);
}

pub fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraph) -> T) -> T {
  let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
  let graph = guard.as_mut().unwrap();
  modifier(graph)
}
