use arena_graph::*;

pub use shader_derives::*;

use std::{
  any::{Any, TypeId},
  collections::HashMap,
  sync::Mutex,
};

mod code_gen;

pub mod builder;
pub mod meta;
pub mod nodes;
pub mod operator;
pub mod provider;
pub mod structor;
pub mod swizzle;
pub mod traits_impl;
pub use builder::*;
pub use meta::*;
pub use nodes::*;
pub use provider::*;
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

#[derive(Copy, Clone)]
pub struct Node<T> {
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
  UBO((&'static UBOMetaInfo, NodeUntyped)),
}

pub struct ShaderGraphCompileResult {
  pub vertex_shader: String,
  pub frag_shader: String,
  pub shader_interface_info: PipelineShaderInterfaceInfo,
}

#[derive(Default)]
pub struct ShaderGraph {
  pub attributes: Vec<(NodeUntyped, usize)>,
  pub vertex_position: Option<Node<Vec4<f32>>>,
  pub vertex_registered: HashMap<TypeId, NodeUntyped>,

  pub struct_define: HashMap<TypeId, String>,

  pub varyings: Vec<(NodeUntyped, usize)>,
  pub frag_outputs: Vec<(NodeUntyped, usize)>,
  pub fragment_registered: HashMap<TypeId, NodeUntyped>,

  pub bindgroups: Vec<ShaderGraphBindGroup>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,

  pub type_id_map: HashMap<TypeId, &'static str>, // totally hack

  pub shader_interface: PipelineShaderInterfaceInfo,
}

pub struct ShaderGraphBindGroup {
  pub inputs: Vec<(ShaderGraphUniformInputType, ShaderStages)>,
}

/// Descriptor of the shader input
#[derive(Clone, Default)]
pub struct PipelineShaderInterfaceInfo {
  // pub bindgroup_layouts: Vec<Vec<rendiation_webgpu::BindGroupLayoutEntry>>,
// pub vertex_state: Option<Vec<rendiation_webgpu::VertexBufferLayout<'static>>>,
// pub preferred_target_states: TargetStates,
// pub primitive_states: PrimitiveState,
}

impl ShaderGraph {
  pub fn compile(&self) -> ShaderGraphCompileResult {
    // do extra naga check;
    let vertex = self.gen_code_vertex();
    let frag = self.gen_code_frag();

    // let naga_vertex_ir = naga::front::glsl::parse_str(
    //   &vertex,
    //   "main",
    //   naga::ShaderStage::Vertex,
    //   HashMap::default(),
    // );
    // let naga_frag_ir = naga::front::glsl::parse_str(
    //   &frag,
    //   "main",
    //   naga::ShaderStage::Fragment,
    //   HashMap::default(),
    // );
    // if naga_vertex_ir.is_err() {
    //   println!("{:?}", naga_vertex_ir);
    //   println!("{:}", vertex);
    // }
    // if naga_frag_ir.is_err() {
    //   println!("{:?}", naga_frag_ir);
    //   println!("{:}", frag);
    // }
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

pub static IN_BUILDING_SHADER_GRAPH: once_cell::sync::Lazy<Mutex<Option<ShaderGraph>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(None));

pub fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraph) -> T) -> T {
  let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
  let graph = guard.as_mut().unwrap();
  modifier(graph)
}

pub fn set_build_graph(g: ShaderGraph) {
  let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
  *guard = Some(g);
}

pub fn take_build_graph() -> ShaderGraph {
  IN_BUILDING_SHADER_GRAPH.lock().unwrap().take().unwrap()
}

pub trait SemanticShaderValue: Any {
  type ValueType;
  const NAME: &'static str = "unnamed";
  const STAGE: ShaderStages;
}

pub fn query<T: SemanticShaderValue>() -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
  todo!()
}

pub fn register<T: SemanticShaderValue>(node: impl Into<Node<T::ValueType>>) {
  todo!()
}

pub fn register_uniform<T>() -> Node<T> {
  todo!()
}

pub fn query_uniform<T>() -> Result<Node<T>, ShaderGraphBuildError> {
  todo!()
}

pub fn set_vertex_out<T>(node: Node<T>) {
  //
}

pub fn set_fragment_out<T>(node: Node<T>) {
  //
}

pub enum ShaderGraphBuildError {
  MissingRequiredDependency,
}

pub trait ShaderGraphBuilder {
  fn build(&self) -> Result<(), ShaderGraphBuildError>;
}

// impl ShaderGraphBuilder for PassDispatcher {
//   fn build(&self) {
//     // set_vertex_in
//   }
// }

// impl ShaderGraphBuilder for Geometry {
//   fn build(&self) {
//     // set_vertex_in
//     // register semantic value
//   }
// }

// impl ShaderGraphBuilder for Camera {
//   fn build(&self) {
//     // query geometry position in;
//     // set project position
//   }
// }

// impl ShaderGraphBuilder for Material {
//   fn build(&self) {
//     // write channel
//   }
// }
