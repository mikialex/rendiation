use crate::*;

pub trait SemanticVertexShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = core::intrinsics::type_name::<Self>();
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
  // built in vertex in
  pub vertex_index: Node<u32>,
  pub instance_index: Node<u32>,

  // user vertex in
  pub(crate) vertex_in: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType, usize)>,
  pub vertex_layouts: Vec<ShaderGraphVertexBufferLayout>,
  pub primitive_state: PrimitiveState,

  // user semantic vertex
  registry: SemanticRegistry,

  // user vertex out
  pub(crate) vertex_out: HashMap<TypeId, (NodeUntyped, PrimitiveShaderValueType, usize)>,
  pub(crate) vertex_out_not_synced_to_fragment: HashSet<TypeId>,
}

impl ShaderGraphVertexBuilder {
  pub(crate) fn new() -> Self {
    set_current_building(ShaderStages::Vertex.into());

    let vertex_index = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexIndexId).insert_graph();

    let instance_index =
      ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexInstanceId).insert_graph();
    set_current_building(None);

    Self {
      vertex_index,
      instance_index,
      vertex_in: Default::default(),
      registry: Default::default(),
      vertex_out: Default::default(),
      vertex_layouts: Default::default(),
      primitive_state: Default::default(),
      vertex_out_not_synced_to_fragment: Default::default(),
    }
  }

  pub fn sync_fragment_out(&mut self, fragment: &mut ShaderGraphFragmentBuilder) {
    let vertex_out = &mut self.vertex_out;
    self
      .vertex_out_not_synced_to_fragment
      .drain()
      .for_each(|id| {
        let (_, ty, index) = vertex_out.get(&id).unwrap();

        set_current_building(ShaderStages::Fragment.into());
        let node = ShaderGraphInputNode::FragmentIn {
          ty: *ty,
          index: *index,
        }
        .insert_graph();
        fragment.registry.register(id, node);
        set_current_building(None);

        fragment.fragment_in.insert(
          id,
          (node, *ty, ShaderVaryingInterpolation::Perspective, *index),
        );
      })
  }

  pub fn query<T: SemanticVertexShaderValue>(
    &self,
  ) -> Result<&NodeMutable<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn query_or_insert_default<T>(&mut self) -> &NodeMutable<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
  {
    if let Ok(n) = self.registry.query(TypeId::of::<T>(), T::NAME) {
      unsafe { std::mem::transmute(n) }
    } else {
      let default: T::ValueType = Default::default();
      self.register::<T>(default)
    }
  }

  pub fn register<T: SemanticVertexShaderValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) -> &NodeMutable<T::ValueType> {
    let n = self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
    unsafe { std::mem::transmute(n) }
  }

  /// return registered location
  pub fn register_vertex_in<T>(&mut self) -> u32
  where
    T: SemanticVertexShaderValue,
    T::ValueType: VertexInShaderGraphNodeType,
  {
    self.register_vertex_in_inner(T::ValueType::PRIMITIVE_TYPE, TypeId::of::<T>())
  }

  /// untyped version
  pub fn register_vertex_in_inner(&mut self, ty: PrimitiveShaderValueType, ty_id: TypeId) -> u32 {
    let index = self.vertex_in.len();
    let node = ShaderGraphInputNode::VertexIn { ty, index }.insert_graph();
    self.registry.register(ty_id, node);

    self
      .vertex_in
      .entry(ty_id)
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
    let id = TypeId::of::<T>();
    self.vertex_out.entry(id).or_insert_with(|| {
      (
        node.cast_untyped_node(),
        <T as SemanticVertexShaderValue>::ValueType::PRIMITIVE_TYPE,
        len,
      )
    });
    self.register::<T>(node);
    self.vertex_out_not_synced_to_fragment.insert(id);
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
