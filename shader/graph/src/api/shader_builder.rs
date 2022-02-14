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

impl<'a> ShaderGraphProvider for &'a [&dyn ShaderGraphProvider] {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for p in *self {
      p.build_vertex(builder)?;
    }
    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for p in *self {
      p.build_fragment(builder)?;
    }
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
  let mut result = vertex_builder.extract();
  result.top_scope_mut().resolve_all_pending();
  let vertex_shader = target.gen_vertex_shader(&mut vertex_builder, result);

  let mut fragment_builder = ShaderGraphFragmentBuilder::create(vertex_builder);
  builder.build_fragment(&mut fragment_builder)?;
  let mut result = fragment_builder.extract();
  result.top_scope_mut().resolve_all_pending();
  let frag_shader = target.gen_fragment_shader(&mut fragment_builder, result);

  Ok(ShaderGraphCompileResult {
    vertex_shader,
    frag_shader,
    bindings: fragment_builder.bindgroups,
  })
}

pub struct ShaderGraphCompileResult {
  pub vertex_shader: String,
  pub frag_shader: String,
  pub bindings: ShaderGraphBindGroupBuilder,
}

pub struct ShaderGraphVertexBuilder {
  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  // built in vertex in
  pub vertex_index: Node<u32>,
  pub instance_index: Node<u32>,

  // user vertex in
  pub(crate) vertex_in: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType, usize)>,

  // user semantic vertex
  registry: SemanticRegistry,

  // built in vertex out
  pub vertex_position: NodeMutable<Vec4<f32>>,

  // user vertex out
  pub(crate) vertex_out: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType, usize)>,
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
  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  // user fragment in
  pub(crate) fragment_in: HashMap<
    TypeId,
    (
      NodeUntyped,
      PrimitiveShaderValueType,
      ShaderVaryingInterpolation,
      usize,
    ),
  >,

  registry: SemanticRegistry,

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
  pub(crate) current_stage: ShaderStages,
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

pub trait SemanticShaderUniform: Any {
  type Node: ShaderGraphNodeType;
  const TYPE: SemanticBinding;
}

impl ShaderGraphBindGroupBuilder {
  #[inline(never)]
  fn register_uniform_inner(
    &mut self,
    type_id: TypeId,
    bindgroup_index: usize,
    ty: ShaderValueType,
  ) -> NodeUntyped {
    if let Ok(node) = self.query_uniform_inner(type_id, bindgroup_index) {
      return node;
    }

    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let node = ShaderGraphNodeData::Input(ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    })
    .insert_graph();

    let (node_vertex, node_fragment) = match self.current_stage {
      ShaderStages::Vertex => (node.handle().into(), None),
      ShaderStages::Fragment => (None, node.handle().into()),
    };

    bindgroup.bindings.push((
      ShaderGraphBindEntry {
        ty,
        node_vertex,
        node_fragment,
      },
      type_id,
    ));

    node
  }

  #[inline(never)]
  fn query_uniform_inner(
    &mut self,
    type_id: TypeId,
    bindgroup_index: usize,
  ) -> Result<NodeUntyped, ShaderGraphBuildError> {
    let current_stage = self.current_stage;
    let bindgroup = &mut self.bindings[bindgroup_index];

    bindgroup
      .bindings
      .iter_mut()
      .enumerate()
      .find(|(_, entry)| entry.1 == type_id)
      .map(|(i, (entry, _))| unsafe {
        let node = match current_stage {
          ShaderStages::Vertex => &mut entry.node_vertex,
          ShaderStages::Fragment => &mut entry.node_fragment,
        };
        node
          .get_or_insert_with(|| {
            ShaderGraphNodeData::Input(ShaderGraphInputNode::Uniform {
              bindgroup_index,
              entry_index: i,
            })
            .insert_graph::<AnyType>()
            .handle()
          })
          .into_node()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  #[inline]
  pub fn register_uniform<T: SemanticShaderUniform>(&mut self) -> Node<T::Node> {
    let node = self.register_uniform_inner(
      TypeId::of::<T>(),
      T::TYPE.binding_index(),
      T::Node::to_type(),
    );
    unsafe { node.cast_type() }
  }

  #[inline]
  pub fn register_uniform_by<T: SemanticShaderUniform>(&mut self, _instance: &T) -> Node<T::Node> {
    self.register_uniform::<T>()
  }

  #[inline]
  pub fn query_uniform<T: SemanticShaderUniform>(
    &mut self,
  ) -> Result<Node<T::Node>, ShaderGraphBuildError> {
    let result = self.query_uniform_inner(TypeId::of::<T>(), T::TYPE.binding_index());
    result.map(|n| unsafe { n.cast_type() })
  }
}

#[derive(Default)]
pub struct SemanticRegistry {
  registered: HashMap<TypeId, NodeMutable<AnyType>>,
}

impl SemanticRegistry {
  pub fn query(&mut self, id: TypeId) -> Result<&NodeMutable<AnyType>, ShaderGraphBuildError> {
    self
      .registered
      .get(&id)
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn register(&mut self, id: TypeId, node: NodeUntyped) {
    self.registered.entry(id).or_insert_with(|| node.mutable());
  }
}

impl ShaderGraphVertexBuilder {
  pub fn create(bindgroups: ShaderGraphBindGroupBuilder) -> Self {
    let builder = ShaderGraphBuilder::default();

    set_build_graph(builder);

    // default position
    let vertex_position = ShaderGraphNodeExpr::Const(ConstNode {
      data: PrimitiveShaderValue::Vec4Float32(Vec4::zero()),
    })
    .insert_graph()
    .mutable();

    let vertex_index =
      ShaderGraphNodeData::Input(ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexIndexId))
        .insert_graph();

    let instance_index = ShaderGraphNodeData::Input(ShaderGraphInputNode::BuiltIn(
      ShaderBuiltIn::VertexInstanceId,
    ))
    .insert_graph();

    Self {
      bindgroups,
      vertex_index,
      instance_index,
      vertex_in: Default::default(),
      registry: Default::default(),
      vertex_position,
      vertex_out: Default::default(),
    }
  }

  pub fn extract(&self) -> ShaderGraphBuilder {
    take_build_graph()
  }

  pub fn query<T: SemanticVertexShaderValue>(
    &mut self,
  ) -> Result<&NodeMutable<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>())
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn register<T: SemanticVertexShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
  }

  pub fn register_vertex_in<T: SemanticVertexGeometryIn>(&mut self) -> Node<T::ValueType> {
    let ty = T::ValueType::to_primitive_type();
    let index = self.vertex_in.len();
    let node =
      ShaderGraphNodeData::Input(ShaderGraphInputNode::VertexIn { ty, index }).insert_graph();
    self
      .vertex_in
      .entry(TypeId::of::<T>())
      .or_insert_with(|| (node.cast_untyped_node(), ty, index));
    node
  }

  pub fn set_vertex_out<T: SemanticVertexFragmentIOValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) {
    let len = self.vertex_out.len();
    self.vertex_out.entry(TypeId::of::<T>()).or_insert_with(|| {
      (
        node.into().cast_untyped_node(),
        T::ValueType::to_primitive_type(),
        len,
      )
    });
  }
}

impl ShaderGraphFragmentBuilder {
  pub fn create(mut vertex: ShaderGraphVertexBuilder) -> Self {
    let builder = ShaderGraphBuilder::default();
    set_build_graph(builder);

    let mut fragment_in = HashMap::default();
    vertex.vertex_out.iter().for_each(|(id, (_, ty, index))| {
      let node = ShaderGraphNodeData::Input(ShaderGraphInputNode::FragmentIn {
        ty: *ty,
        index: *index,
      })
      .insert_graph();
      fragment_in.insert(
        *id,
        (node, *ty, ShaderVaryingInterpolation::Perspective, *index),
      );
    });

    vertex.current_stage = ShaderStages::Fragment;

    Self {
      bindgroups: vertex.bindgroups,
      fragment_in,
      registry: Default::default(),
      frag_output: Default::default(),
    }
  }

  pub fn discard(&self) {
    ShaderSideEffectNode::Termination.insert_graph_bottom();
  }

  pub fn query<T: SemanticFragmentShaderValue>(
    &mut self,
  ) -> Result<&NodeMutable<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>())
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn register<T: SemanticFragmentShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
  }

  pub fn get_fragment_in<T: SemanticVertexFragmentIOValue>(
    &mut self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .fragment_in
      .get(&TypeId::of::<T>())
      .map(|(n, _, _, _)| unsafe { (*n).cast_type() })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn set_fragment_out(&mut self, channel: usize, node: Node<Vec4<f32>>) {
    while channel >= self.frag_output.len() {
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
