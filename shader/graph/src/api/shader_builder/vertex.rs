use crate::*;

pub trait SemanticVertexShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
}

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ShaderGraphVertexBufferLayout {
  /// The stride, in bytes, between elements of this buffer.
  pub array_stride: BufferAddress,
  /// How often this vertex buffer is "stepped" forward.
  pub step_mode: VertexStepMode,
  /// The list of attributes which comprise a single vertex.
  pub attributes: Vec<VertexAttribute>,
}

pub struct ShaderGraphVertexBuilder {
  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  // built in vertex in
  pub vertex_index: Node<u32>,
  pub instance_index: Node<u32>,

  // user vertex in
  pub(crate) vertex_in: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType, usize)>,
  pub vertex_layouts: Vec<ShaderGraphVertexBufferLayout>,
  pub primitive_state: PrimitiveState,

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

    let vertex_index = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexIndexId).insert_graph();

    let instance_index =
      ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexInstanceId).insert_graph();

    Self {
      bindgroups,
      vertex_index,
      instance_index,
      vertex_in: Default::default(),
      registry: Default::default(),
      vertex_position,
      vertex_out: Default::default(),
      vertex_layouts: Default::default(),
      primitive_state: Default::default(),
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

  /// return registered location
  pub fn register_vertex_in<T>(&mut self) -> u32
  where
    T: SemanticVertexShaderValue,
    T::ValueType: VertexInShaderGraphNodeType,
  {
    let ty = T::ValueType::to_primitive_type();
    let index = self.vertex_in.len();
    let node = ShaderGraphInputNode::VertexIn { ty, index }.insert_graph();
    self.register::<T>(node);

    self
      .vertex_in
      .entry(TypeId::of::<T>())
      .or_insert_with(|| (node.cast_untyped_node(), ty, index));

    index as u32
  }

  pub fn push_vertex_layout(&mut self, layout: ShaderGraphVertexBufferLayout) {
    self.vertex_layouts.push(layout)
  }

  pub fn set_vertex_out<T>(
    &mut self,
    node: impl Into<Node<<T as SemanticVertexShaderValue>::ValueType>>,
  ) where
    T: SemanticVertexShaderValue,
    T: SemanticFragmentShaderValue,
    <T as SemanticVertexShaderValue>::ValueType: PrimitiveShaderGraphNodeType,
    T: SemanticFragmentShaderValue<ValueType = <T as SemanticVertexShaderValue>::ValueType>,
  {
    let len = self.vertex_out.len();
    let node = node.into();
    self.vertex_out.entry(TypeId::of::<T>()).or_insert_with(|| {
      (
        node.cast_untyped_node(),
        <T as SemanticVertexShaderValue>::ValueType::to_primitive_type(),
        len,
      )
    });
    self.register::<T>(node);
  }

  pub fn register_vertex<V>(&mut self, step_mode: VertexStepMode)
  where
    V: ShaderGraphVertexInProvider,
  {
    V::provide_layout_and_vertex_in(self, step_mode)
  }
}

pub trait ShaderGraphVertexInProvider {
  fn provide_layout_and_vertex_in(
    builder: &mut ShaderGraphVertexBuilder,
    step_mode: VertexStepMode,
  );
}
