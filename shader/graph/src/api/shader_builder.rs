use std::{
  any::{Any, TypeId},
  cell::UnsafeCell,
  collections::HashMap,
};

use crate::*;

pub trait SemanticVertexGeometryIn: Any {
  type ValueType: PrimitiveShaderGraphNodeType;
  const NAME: &'static str = "unnamed";
}

pub trait SemanticVertexShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = "unnamed";
}

pub trait SemanticVertexFragmentIOValue: Any {
  type ValueType: PrimitiveShaderGraphNodeType;
  const NAME: &'static str = "unnamed";
}

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = "unnamed";
}

#[derive(Debug)]
pub enum ShaderGraphBuildError {
  MissingRequiredDependency,
}

/// The reason why we use two function is that the build process
/// require to generate two separate root scope: two entry main function;
pub trait ShaderGraphProvider {
  fn build_vertex(
    &self,
    _builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
  }
  fn build_fragment(
    &self,
    _builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
  }
}

/// entry
pub fn build_shader(
  builder: &dyn ShaderGraphProvider,
  target: &dyn ShaderGraphCodeGenTarget,
) -> Result<ShaderGraphCompileResult, ShaderGraphBuildError> {
  let bindgroup_builder = ShaderGraphBindGroupBuilder::default();

  let mut vertex_builder = ShaderGraphVertexBuilder::create(bindgroup_builder);
  builder.build_vertex(&mut vertex_builder)?;
  let result = vertex_builder.extract();
  let vertex_shader = target.gen_vertex_shader(&mut vertex_builder, result);

  let mut fragment_builder = ShaderGraphFragmentBuilder::create(vertex_builder);
  builder.build_fragment(&mut fragment_builder)?;
  let result = fragment_builder.extract();
  let frag_shader = target.gen_fragment_shader(&mut fragment_builder, result);

  Ok(ShaderGraphCompileResult {
    vertex_shader,
    frag_shader,
    states: Default::default(),
    bindings: fragment_builder.bindgroups,
  })
}

pub struct ShaderGraphCompileResult {
  pub vertex_shader: String,
  pub frag_shader: String,
  pub states: PipelineShaderInterfaceInfo,
  pub bindings: ShaderGraphBindGroupBuilder,
}

pub struct ShaderGraphVertexBuilder {
  // states
  pub shader_interface: PipelineShaderInterfaceInfo,

  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  // built in vertex in
  pub vertex_index: Node<u32>,
  pub instance_index: Node<u32>,

  // user vertex in
  vertex_in: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType)>,

  // user semantic vertex
  vertex_registered: HashMap<TypeId, NodeUntyped>,

  // built in vertex out
  pub vertex_point_size: Node<Mutable<f32>>,
  pub vertex_position: Node<Mutable<Vec4<f32>>>,

  // user vertex out
  pub(crate) vertex_out: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType)>,

  pending_resolve: HashMap<TypeId, PendingResolve>,
}

struct PendingResolve {
  unresolved: Rc<Cell<ShaderGraphNodeRawHandleUntyped>>,
  depend_by: Vec<ShaderGraphNodeRawHandleUntyped>,
}

impl std::ops::Deref for ShaderGraphVertexBuilder {
  type Target = ShaderGraphBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

impl std::ops::DerefMut for ShaderGraphVertexBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bindgroups
  }
}

pub struct ShaderGraphFragmentBuilder {
  // states
  pub shader_interface: PipelineShaderInterfaceInfo,

  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  pub varying_info: Vec<ShaderVaryingValueInfo>,

  // user fragment in
  fragment_in: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType)>,

  fragment_registered: HashMap<TypeId, NodeUntyped>,

  pub frag_output: Vec<Node<Vec4<f32>>>,
}

impl std::ops::Deref for ShaderGraphFragmentBuilder {
  type Target = ShaderGraphBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

impl std::ops::DerefMut for ShaderGraphFragmentBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bindgroups
  }
}

pub struct ShaderGraphBindGroupBuilder {
  pub current_stage: ShaderStages,
  pub bindings: Vec<ShaderGraphBindGroup>,
}

impl Default for ShaderGraphBindGroupBuilder {
  fn default() -> Self {
    Self {
      current_stage: ShaderStages::Vertex,
      bindings: vec![Default::default(); 5],
    }
  }
}

#[derive(Clone, Copy)]
pub enum SemanticBinding {
  Global,
  Camera,
  Pass,
  Material,
  Object,
}

impl SemanticBinding {
  pub fn binding_index(&self) -> usize {
    match self {
      SemanticBinding::Global => 4,
      SemanticBinding::Camera => 3,
      SemanticBinding::Pass => 2,
      SemanticBinding::Material => 1,
      SemanticBinding::Object => 0,
    }
  }
}

pub trait SemanticShaderUniform: ShaderGraphNodeType {
  const TYPE: SemanticBinding;
}

impl ShaderGraphBindGroupBuilder {
  pub fn register_uniform<T: SemanticShaderUniform>(&mut self) -> Node<T> {
    if let Ok(node) = self.query_uniform() {
      return node;
    }

    let bindgroup_index = T::TYPE.binding_index();
    let bindgroup = &mut self.bindings[bindgroup_index];
    let type_id = TypeId::of::<T>();

    let ty = T::to_type();
    let entry_index = bindgroup.bindings.len();
    let node: Node<T> = ShaderGraphNodeData::Input(ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    })
    .insert_graph();

    bindgroup.bindings.push((
      ShaderGraphBindEntry {
        ty,
        used_in_vertex: self.current_stage == ShaderStages::Vertex,
        used_in_fragment: self.current_stage == ShaderStages::Fragment,
        node: node.handle().cast_untyped(),
      },
      type_id,
    ));

    node
  }

  pub fn query_uniform<T: SemanticShaderUniform>(
    &mut self,
  ) -> Result<Node<T>, ShaderGraphBuildError> {
    let bindgroup_index = T::TYPE.binding_index();
    let bindgroup = &mut self.bindings[bindgroup_index];
    let type_id = TypeId::of::<T>();
    let used_in_vertex = self.current_stage == ShaderStages::Vertex;
    let used_in_fragment = self.current_stage == ShaderStages::Fragment;

    bindgroup
      .bindings
      .iter_mut()
      .find(|entry| entry.1 == type_id)
      .map(|(entry, _)| unsafe {
        entry.used_in_vertex |= used_in_vertex;
        entry.used_in_fragment |= used_in_fragment;
        entry.node.cast_type().into()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }
}

/// Descriptor of the shader input
#[derive(Clone, Default)]
pub struct PipelineShaderInterfaceInfo {
  // pub bindgroup_layouts: Vec<Vec<rendiation_webgpu::BindGroupLayoutEntry>>,
// pub vertex_state: Option<Vec<rendiation_webgpu::VertexBufferLayout<'static>>>,
// pub preferred_target_states: TargetStates,
// pub primitive_states: PrimitiveState,
}

impl ShaderGraphVertexBuilder {
  pub fn create(bindgroups: ShaderGraphBindGroupBuilder) -> Self {
    let mut builder = ShaderGraphBuilder::default();

    let vertex_point_size = ShaderGraphNodeExpr::Const(ConstNode {
      data: PrimitiveShaderValue::Float32(1.),
    })
    .insert_into_graph(&mut builder);

    let vertex_position = ShaderGraphNodeExpr::Const(ConstNode {
      data: PrimitiveShaderValue::Vec4Float32(Vec4::zero()),
    })
    .insert_into_graph(&mut builder);

    let vertex_index =
      ShaderGraphNodeData::Input(ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexIndexId))
        .insert_into_graph(&mut builder);

    let instance_index = ShaderGraphNodeData::Input(ShaderGraphInputNode::BuiltIn(
      ShaderBuiltIn::VertexInstanceId,
    ))
    .insert_into_graph(&mut builder);

    set_build_graph(builder);

    Self {
      shader_interface: Default::default(),
      bindgroups,
      vertex_index,
      instance_index,
      vertex_in: Default::default(),
      vertex_registered: Default::default(),
      vertex_point_size,
      vertex_position,
      vertex_out: Default::default(),
      pending_resolve: Default::default(),
    }
  }

  pub fn extract(&self) -> ShaderGraphBuilder {
    take_build_graph()
  }

  pub fn query_last<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: Default,
  {
    let cell = &self
      .pending_resolve
      .entry(TypeId::of::<T>())
      .or_insert_with(|| todo!())
      .unresolved;
    let cell: &Rc<Cell<ShaderGraphNodeRawHandle<T::ValueType>>> =
      unsafe { std::mem::transmute(cell) };
    Node {
      handle: cell.clone(),
    }
  }

  pub fn resolve_all_pending(&mut self) {
    let registered = &self.vertex_registered;
    self.pending_resolve.drain().for_each(|(id, to_resolve)| {
      if let Some(target) = registered.get(&id) {
        //
      }
    })
  }

  pub fn query<T: SemanticVertexShaderValue>(
    &mut self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .vertex_registered
      .get(&TypeId::of::<T>())
      .map(|node| {
        let n: &Node<Mutable<T::ValueType>> = unsafe { std::mem::transmute(node) };
        n.get()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn register<T: SemanticVertexShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self
      .vertex_registered
      .entry(TypeId::of::<T>())
      .or_insert_with(|| node.into().cast_untyped_node());
  }

  pub fn register_vertex_in<T: SemanticVertexGeometryIn>(&mut self) -> Node<T::ValueType> {
    let ty = T::ValueType::to_primitive_type();
    let node = ShaderGraphNodeData::Input(ShaderGraphInputNode::VertexIn {
      ty,
      index: self.vertex_in.len(),
    })
    .insert_graph();
    self
      .vertex_in
      .entry(TypeId::of::<T>())
      .or_insert_with(|| (node.cast_untyped_node(), ty));
    node
  }

  pub fn set_vertex_out<T: SemanticVertexFragmentIOValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) {
    self.vertex_out.entry(TypeId::of::<T>()).or_insert_with(|| {
      (
        node.into().cast_untyped_node(),
        T::ValueType::to_primitive_type(),
      )
    });
  }
}

impl ShaderGraphFragmentBuilder {
  pub fn create(mut vertex: ShaderGraphVertexBuilder) -> Self {
    // todo register vertex out to frag in

    let builder = ShaderGraphBuilder::default();

    set_build_graph(builder);

    vertex.current_stage = ShaderStages::Fragment;

    Self {
      shader_interface: Default::default(),
      bindgroups: vertex.bindgroups,
      varying_info: Default::default(),
      fragment_in: Default::default(),
      fragment_registered: Default::default(),
      frag_output: Default::default(),
    }
  }

  pub fn query<T: SemanticVertexShaderValue>(
    &mut self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .fragment_registered
      .get(&TypeId::of::<T>())
      .map(|node| {
        let n: &Node<Mutable<T::ValueType>> = unsafe { std::mem::transmute(node) };
        n.get()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn register<T: SemanticVertexShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self
      .fragment_registered
      .entry(TypeId::of::<T>())
      .or_insert_with(|| node.into().cast_untyped_node());
  }

  pub fn get_fragment_in<T: SemanticVertexFragmentIOValue>(
    &mut self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .fragment_in
      .get(&TypeId::of::<T>())
      .map(|node| {
        let n: &Node<Mutable<T::ValueType>> = unsafe { std::mem::transmute(node) };
        n.get()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn set_fragment_out(&mut self, channel: usize, node: Node<Vec4<f32>>) {
    while channel <= self.frag_output.len() {
      self.frag_output.push(consts(Vec4::zero()));
    }
    self.frag_output[channel] = node;
  }

  pub fn extract(&self) -> ShaderGraphBuilder {
    take_build_graph()
  }
}

pub struct SuperUnsafeCell<T> {
  pub data: UnsafeCell<T>,
}

impl<T> SuperUnsafeCell<T> {
  pub fn new(v: T) -> Self {
    Self {
      data: UnsafeCell::new(v),
    }
  }
  #[allow(clippy::mut_from_ref)]
  pub fn get_mut(&self) -> &mut T {
    unsafe { &mut *(self.data.get()) }
  }
  pub fn get(&self) -> &T {
    unsafe { &*(self.data.get()) }
  }
}

unsafe impl<T> Sync for SuperUnsafeCell<T> {}
unsafe impl<T> Send for SuperUnsafeCell<T> {}

static IN_BUILDING_SHADER_GRAPH: once_cell::sync::Lazy<
  SuperUnsafeCell<Option<ShaderGraphBuilder>>,
> = once_cell::sync::Lazy::new(|| SuperUnsafeCell::new(None));

pub(crate) fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraphBuilder) -> T) -> T {
  let graph = IN_BUILDING_SHADER_GRAPH.get_mut().as_mut().unwrap();
  modifier(graph)
}

pub(crate) fn set_build_graph(g: ShaderGraphBuilder) {
  IN_BUILDING_SHADER_GRAPH.get_mut().replace(g);
}

pub(crate) fn take_build_graph() -> ShaderGraphBuilder {
  IN_BUILDING_SHADER_GRAPH.get_mut().take().unwrap()
}

/// built in semantics
pub struct VertexIndex;
impl SemanticVertexShaderValue for VertexIndex {
  type ValueType = u32;
}
