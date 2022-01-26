use std::{any::TypeId, collections::HashMap, sync::Mutex};

use crate::*;

pub struct ShaderGraphCompileResult {
  pub vertex_shader: String,
  pub frag_shader: String,
  pub shader_interface_info: PipelineShaderInterfaceInfo,
}

pub struct ShaderGraphShaderBuilder {
  // states
  pub shader_interface: PipelineShaderInterfaceInfo,

  // uniforms
  pub bindgroups: Vec<ShaderGraphBindGroup>,

  // built in vertex in
  pub vertex_index: Node<u32>,
  pub instance_index: Node<u32>,

  // user vertex in
  pub vertex_in: Vec<(NodeUntyped, PrimitiveShaderValueType)>,

  // user semantic vertex
  pub vertex_registered: HashMap<TypeId, NodeUntyped>,

  // built in vertex out
  pub vertex_point_size: Node<Mutable<f32>>,
  pub vertex_position: Node<Mutable<f32>>,

  // user vertex out
  pub vertex_out: Vec<(NodeUntyped, PrimitiveShaderValueType)>,

  pub varying_info: Vec<ShaderVaryingValueInfo>,

  // user fragment in
  pub fragment_in: Vec<(NodeUntyped, PrimitiveShaderValueType)>,

  pub fragment_registered: HashMap<TypeId, NodeUntyped>,

  pub frag_output: Vec<Node<Vec4<f32>>>,
}

pub enum ShaderVaryingInterpolation {
  Flat,
  Perspective,
}

pub struct ShaderVaryingValueInfo {
  interpolation: usize,
  ty: PrimitiveShaderValueType,
}

pub enum ShaderGraphBindgroupEntry {
  Sampler(Node<ShaderSampler>),
  Texture(Node<ShaderTexture>),
  UBO((&'static ShaderStructMetaInfo, NodeUntyped)),
}

pub struct ShaderGraphBindGroup {
  pub inputs: Vec<(ShaderGraphBindgroupEntry, ShaderStages)>,
}

/// Descriptor of the shader input
#[derive(Clone, Default)]
pub struct PipelineShaderInterfaceInfo {
  // pub bindgroup_layouts: Vec<Vec<rendiation_webgpu::BindGroupLayoutEntry>>,
// pub vertex_state: Option<Vec<rendiation_webgpu::VertexBufferLayout<'static>>>,
// pub preferred_target_states: TargetStates,
// pub primitive_states: PrimitiveState,
}

impl ShaderGraphShaderBuilder {
  pub fn create() -> Self {
    todo!();
  }

  pub fn compile(&self) -> ShaderGraphCompileResult {
    todo!()
    // // do extra naga check;
    // let vertex = self.gen_code_vertex();
    // let frag = self.gen_code_frag();

    // // let naga_vertex_ir = naga::front::glsl::parse_str(
    // //   &vertex,
    // //   "main",
    // //   naga::ShaderStage::Vertex,
    // //   HashMap::default(),
    // // );
    // // let naga_frag_ir = naga::front::glsl::parse_str(
    // //   &frag,
    // //   "main",
    // //   naga::ShaderStage::Fragment,
    // //   HashMap::default(),
    // // );
    // // if naga_vertex_ir.is_err() {
    // //   println!("{:?}", naga_vertex_ir);
    // //   println!("{:}", vertex);
    // // }
    // // if naga_frag_ir.is_err() {
    // //   println!("{:?}", naga_frag_ir);
    // //   println!("{:}", frag);
    // // }
    // ShaderGraphCompileResult {
    //   vertex_shader: vertex,
    //   frag_shader: frag,
    //   shader_interface_info: self.shader_interface.clone(),
    // }
  }
}

pub static IN_BUILDING_SHADER: once_cell::sync::Lazy<Mutex<Option<ShaderGraphShaderBuilder>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(None));

pub fn modify_shader_builder<T>(modifier: impl FnOnce(&mut ShaderGraphShaderBuilder) -> T) -> T {
  let mut guard = IN_BUILDING_SHADER.lock().unwrap();
  let builder = guard.as_mut().unwrap();
  modifier(builder)
}

pub static IN_BUILDING_SHADER_GRAPH: once_cell::sync::Lazy<Mutex<Option<ShaderGraphBuilder>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(None));

pub fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraphBuilder) -> T) -> T {
  let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
  let graph = guard.as_mut().unwrap();
  modifier(graph)
}

pub fn set_build_graph(g: ShaderGraphBuilder) {
  let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
  *guard = Some(g);
}

pub fn take_build_graph() -> ShaderGraphBuilder {
  IN_BUILDING_SHADER_GRAPH.lock().unwrap().take().unwrap()
}

pub fn query<T: SemanticShaderValue>() -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
  modify_shader_builder(|builder| {
    let registry = match T::STAGE {
      ShaderStages::Vertex => &mut builder.vertex_registered,
      ShaderStages::Fragment => &mut builder.fragment_registered,
    };

    registry
      .get(&TypeId::of::<T>())
      .map(|node| {
        let n: &Node<Mutable<T::ValueType>> = unsafe { std::mem::transmute(node) };
        n.get()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  })
}

pub struct VertexIndex;
impl SemanticShaderValue for VertexIndex {
  type ValueType = u32;
  const STAGE: ShaderStages = ShaderStages::Vertex;
}
pub fn query_built_in<T: SemanticShaderValue>() -> Node<T::ValueType> {
  todo!()
}

pub fn register<T: SemanticShaderValue>(node: impl Into<Node<T::ValueType>>) {
  modify_shader_builder(|builder| {
    let registry = match T::STAGE {
      ShaderStages::Vertex => &mut builder.vertex_registered,
      ShaderStages::Fragment => &mut builder.fragment_registered,
    };

    registry
      .entry(TypeId::of::<T>())
      .or_insert_with(|| node.into().cast_untyped_node());
  })
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

pub fn set_fragment_out<T>(channel: usize, node: Node<T>) {
  // modify_shader_builder(|builder| builder.frag_output.set)
}
