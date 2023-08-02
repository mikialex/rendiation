use crate::*;

pub trait SemanticVertexShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = std::any::type_name::<Self>();
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
  // user vertex in
  pub vertex_in: FastHashMap<TypeId, VertexIOInfo>,
  pub vertex_layouts: Vec<ShaderGraphVertexBufferLayout>,
  pub primitive_state: PrimitiveState,

  // user semantic vertex
  pub(crate) registry: SemanticRegistry,

  // user vertex out
  pub vertex_out: FastHashMap<TypeId, VertexIOInfo>,
  pub(crate) vertex_out_not_synced_to_fragment: FastHashSet<TypeId>,
}

#[derive(Copy, Clone)]
pub struct VertexIOInfo {
  pub node: NodeUntyped,
  pub ty: PrimitiveShaderValueType,
  pub location: usize,
}

impl ShaderGraphVertexBuilder {
  pub(crate) fn new() -> Self {
    let mut result = Self {
      vertex_in: Default::default(),
      registry: Default::default(),
      vertex_out: Default::default(),
      vertex_layouts: Default::default(),
      primitive_state: PrimitiveState {
        cull_mode: Some(Face::Back),
        ..Default::default()
      },
      vertex_out_not_synced_to_fragment: Default::default(),
    };

    set_current_building(ShaderStages::Vertex.into());

    let vertex_index = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexIndexId).insert_graph();
    result.register::<VertexIndex>(vertex_index);

    let instance_index =
      ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::VertexInstanceId).insert_graph();
    result.register::<VertexInstanceIndex>(instance_index);

    set_current_building(None);

    result
  }

  pub fn sync_fragment_out(&mut self, fragment: &mut ShaderGraphFragmentBuilder) {
    let vertex_out = &mut self.vertex_out;
    self
      .vertex_out_not_synced_to_fragment
      .drain()
      .for_each(|id| {
        let VertexIOInfo { ty, location, .. } = *vertex_out.get(&id).unwrap();

        set_current_building(ShaderStages::Fragment.into());
        let node = ShaderGraphInputNode::FragmentIn { ty, location }.insert_graph();
        fragment.registry.register(id, node);
        set_current_building(None);

        fragment.fragment_in.insert(
          id,
          (node, ty, ShaderVaryingInterpolation::Perspective, location),
        );
      })
  }

  pub fn registry(&self) -> &SemanticRegistry {
    &self.registry
  }

  pub fn query<T: SemanticVertexShaderValue>(
    &self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
  {
    if let Ok(n) = self.registry.query(TypeId::of::<T>(), T::NAME) {
      unsafe { n.cast_type() }
    } else {
      let default: T::ValueType = Default::default();
      self.register::<T>(default)
    }
  }

  pub fn register<T: SemanticVertexShaderValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) -> Node<T::ValueType> {
    let n = self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
    unsafe { n.cast_type() }
  }

  /// return registered location
  pub fn register_vertex_in<T>(&mut self) -> u32
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
  {
    self.register_vertex_in_inner(T::ValueType::PRIMITIVE_TYPE, TypeId::of::<T>())
  }

  /// untyped version
  pub fn register_vertex_in_inner(&mut self, ty: PrimitiveShaderValueType, ty_id: TypeId) -> u32 {
    let location = self.vertex_in.len();
    let node = ShaderGraphInputNode::VertexIn { ty, location }.insert_graph();
    self.registry.register(ty_id, node);

    self.vertex_in.entry(ty_id).or_insert_with(|| VertexIOInfo {
      node: node.cast_untyped_node(),
      ty,
      location,
    });

    location as u32
  }

  pub fn push_vertex_layout(&mut self, layout: ShaderGraphVertexBufferLayout) {
    self.vertex_layouts.push(layout)
  }

  pub fn push_single_vertex_layout<T>(&mut self, step_mode: VertexStepMode)
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType + VertexInBuilder,
  {
    let mut builder = AttributesListBuilder::default();
    T::ValueType::build_attribute::<T>(&mut builder, self);
    builder.build(self, step_mode);
  }

  pub fn set_vertex_out<T>(&mut self, node: impl Into<Node<T::ValueType>>)
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
  {
    let location = self.vertex_out.len();
    let node = node.into();
    let id = TypeId::of::<T>();
    self.vertex_out.entry(id).or_insert_with(|| VertexIOInfo {
      node: node.cast_untyped_node(),
      ty: T::ValueType::PRIMITIVE_TYPE,
      location,
    });
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

#[derive(Default)]
pub struct AttributesListBuilder {
  inner: Vec<VertexAttribute>,
  byte_size_all: u64,
}

impl AttributesListBuilder {
  pub fn push(&mut self, format: VertexFormat, shader_location: u32) {
    let size = format.size();
    let att = VertexAttribute {
      format,
      offset: self.byte_size_all,
      shader_location,
    };
    self.inner.push(att);
    self.byte_size_all += size;
  }

  pub fn build(self, builder: &mut ShaderGraphVertexBuilder, step_mode: VertexStepMode) {
    let layout = ShaderGraphVertexBufferLayout {
      array_stride: self.byte_size_all,
      step_mode,
      attributes: self.inner,
    };
    builder.push_vertex_layout(layout);
  }
}

pub trait VertexInBuilder {
  fn build_attribute<S>(
    builder: &mut AttributesListBuilder,
    vertex_builder: &mut ShaderGraphVertexBuilder,
  ) where
    S: SemanticVertexShaderValue<ValueType = Self>;
}

impl<T: VertexInShaderGraphNodeType> VertexInBuilder for T {
  fn build_attribute<S>(
    builder: &mut AttributesListBuilder,
    vertex_builder: &mut ShaderGraphVertexBuilder,
  ) where
    S: SemanticVertexShaderValue<ValueType = Self>,
  {
    builder.push(
      T::to_vertex_format(),
      vertex_builder.register_vertex_in::<S>(),
    )
  }
}

impl VertexInBuilder for Mat4<f32> {
  #[rustfmt::skip]
  fn build_attribute<S>(
    builder: &mut AttributesListBuilder,
    vertex_builder: &mut ShaderGraphVertexBuilder,
  ) where
    S: SemanticVertexShaderValue<ValueType = Self>,
  {
    let format = Vec4::<f32>::to_vertex_format();

    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 0>>());
    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 1>>());
    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 2>>());
    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 3>>());

    let c1 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 0>>().unwrap();
    let c2 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 1>>().unwrap();
    let c3 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 2>>().unwrap();
    let c4 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 3>>().unwrap();

    let mat: Node<Self> = (c1, c2, c3, c4).into();
    vertex_builder.register::<S>(mat);
  }
}

struct SemanticShaderMat4VertexInColum<S, const N: usize> {
  phantom: PhantomData<S>,
}

impl<S: 'static, const N: usize> SemanticVertexShaderValue
  for SemanticShaderMat4VertexInColum<S, N>
{
  type ValueType = Vec4<f32>;
}
