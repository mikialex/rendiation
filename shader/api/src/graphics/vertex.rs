use crate::*;

pub trait SemanticVertexShaderValue: Any {
  type ValueType: ShaderNodeType;
  const NAME: &'static str = std::any::type_name::<Self>();
}

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ShaderVertexBufferLayout {
  /// The stride, in bytes, between elements of this buffer.
  pub array_stride: BufferAddress,
  /// How often this vertex buffer is "stepped" forward.
  pub step_mode: VertexStepMode,
  /// The list of attributes which comprise a single vertex.
  pub attributes: Vec<VertexAttribute>,
}

pub struct ShaderVertexBuilder<'a> {
  pub(crate) internal: &'a mut dyn AbstractShaderVertexBuilder,
}

impl<'a> AbstractShaderVertexBuilder for ShaderVertexBuilder<'a> {
  fn task_mesh_shader(&mut self) -> Option<&mut ShaderTaskMeshBuilderGroup> {
    self.internal.task_mesh_shader()
  }
  fn set_current_building(&mut self) {
    self.internal.set_current_building();
  }
  fn vertex_shader(&mut self) -> Option<&mut ShaderRawVertexBuilder> {
    self.internal.vertex_shader()
  }

  fn finalize_write(&mut self) {
    self.internal.finalize_write();
  }
  fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    self.internal.sync_fragment_out(fragment);
  }

  fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    interpolation: ShaderInterpolation,
  ) {
    self
      .internal
      .set_vertex_out_impl(ty_id, ty, node, interpolation);
  }

  fn primitive_state(&mut self) -> &mut PrimitiveState {
    self.internal.primitive_state()
  }

  fn registry(&mut self) -> &mut SemanticRegistry {
    self.internal.registry()
  }

  fn error(&mut self, err: ShaderBuildError) {
    self.internal.error(err);
  }
}

pub struct ShaderRawVertexBuilder {
  // user vertex in
  pub vertex_in: FastHashMap<TypeId, VertexIOInfo>,
  pub vertex_layouts: Vec<ShaderVertexBufferLayout>,
  pub primitive_state: PrimitiveState,

  // user semantic vertex
  registry: SemanticRegistry,

  io_mapping: ShapeFragmentIOMapping,
  // user vertex out
  pub(crate) errors: ErrorSink,
}

#[derive(Copy, Clone)]
pub struct VertexIOInfo {
  pub node: ShaderNodeRawHandle,
  pub ty: PrimitiveShaderValueType,
  pub location: usize,
}

pub fn default_primitive_state() -> PrimitiveState {
  PrimitiveState {
    cull_mode: Some(Face::Back),
    ..Default::default()
  }
}

impl ShaderRawVertexBuilder {
  pub(crate) fn new(errors: ErrorSink) -> Self {
    Self {
      vertex_in: Default::default(),
      registry: Default::default(),
      vertex_layouts: Default::default(),
      primitive_state: default_primitive_state(),
      io_mapping: Default::default(),
      errors,
    }
  }

  /// currently we all depend on ClipPosition in semantic registry to provide the final result
  /// this behavior will be changed in the future;
  pub fn finalize_position_write(&mut self) {
    let position = self.query_or_insert_default::<ClipPosition>();
    call_shader_api(|api| {
      let target = api.define_vertex_position_output();
      api.store(position.handle(), target)
    });
  }

  /// return registered location
  pub fn register_vertex_in<T>(&mut self) -> u32
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.register_vertex_in_inner(T::ValueType::primitive_ty(), TypeId::of::<T>())
  }

  /// untyped version
  pub fn register_vertex_in_inner(&mut self, ty: PrimitiveShaderValueType, ty_id: TypeId) -> u32 {
    let location = self.vertex_in.len();
    let node = ShaderInputNode::UserDefinedIn {
      ty,
      location,
      interpolation: None,
    }
    .insert_api();
    self.registry.register_raw(ty_id, node);

    assert!(!self.vertex_in.contains_key(&ty_id));

    self.vertex_in.entry(ty_id).or_insert_with(|| VertexIOInfo {
      node: node.handle(),
      ty,
      location,
    });

    location as u32
  }

  pub fn push_vertex_layout(&mut self, layout: ShaderVertexBufferLayout) {
    self.vertex_layouts.push(layout)
  }

  pub fn push_single_vertex_layout<T>(&mut self, step_mode: VertexStepMode)
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType + VertexInBuilder,
  {
    let mut builder = AttributesListBuilder::default();
    T::ValueType::build_attribute::<T>(&mut builder, self);
    builder.build(self, step_mode);
  }

  pub fn register_vertex<V>(&mut self, step_mode: VertexStepMode)
  where
    V: ShaderVertexInProvider,
  {
    V::provide_layout_and_vertex_in(self, step_mode)
  }
}

#[derive(Default)]
pub struct ShapeFragmentIOMapping {
  pub vertex_out: FastHashMap<TypeId, (VertexIOInfo, ShaderInterpolation)>,
  pub vertex_out_not_synced_to_fragment: FastHashSet<TypeId>,
}

impl ShapeFragmentIOMapping {
  pub fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    mut interpolation: ShaderInterpolation,
    create_node: &dyn Fn(ShaderInterpolation) -> ShaderNodeRawHandle,
  ) {
    let location = self.vertex_out.len();
    self.vertex_out.entry(ty_id).or_insert_with(|| {
      if !ty.vertex_out_could_interpolated() {
        interpolation = ShaderInterpolation::Flat
      }
      let node = create_node(interpolation);

      (VertexIOInfo { node, ty, location }, interpolation)
    });

    self.vertex_out_not_synced_to_fragment.insert(ty_id);
  }

  pub fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    let vertex_out = &mut self.vertex_out;
    self
      .vertex_out_not_synced_to_fragment
      .drain()
      .for_each(|id| {
        let (VertexIOInfo { ty, location, .. }, interpolation) = *vertex_out.get(&id).unwrap();

        set_current_building(ShaderStage::Fragment.into());
        let node = ShaderInputNode::UserDefinedIn {
          ty,
          location,
          interpolation: Some(interpolation),
        }
        .insert_api();
        fragment.registry.register_raw(id, node);
        set_current_building(None);

        fragment
          .fragment_in
          .insert(id, (node, ty, interpolation, location));
      })
  }
}

impl AbstractShaderVertexBuilder for ShaderRawVertexBuilder {
  fn task_mesh_shader(&mut self) -> Option<&mut ShaderTaskMeshBuilderGroup> {
    None
  }
  fn set_current_building(&mut self) {
    set_current_building(ShaderStage::Vertex.into());
  }

  fn vertex_shader(&mut self) -> Option<&mut ShaderRawVertexBuilder> {
    Some(self)
  }

  fn finalize_write(&mut self) {
    self.finalize_position_write();
  }
  fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    self.io_mapping.sync_fragment_out(fragment);
  }

  fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    interpolation: ShaderInterpolation,
  ) {
    self
      .io_mapping
      .set_vertex_out_impl(ty_id, ty, interpolation, &|interpolation| {
        let target = call_shader_api(|api| api.define_next_vertex_output(ty, Some(interpolation)));
        call_shader_api(|api| api.store(node.handle(), target));
        target
      });
  }

  fn primitive_state(&mut self) -> &mut PrimitiveState {
    &mut self.primitive_state
  }

  fn registry(&mut self) -> &mut SemanticRegistry {
    &mut self.registry
  }

  fn error(&mut self, err: ShaderBuildError) {
    self.errors.push(err);
  }
}

pub trait ShaderVertexInProvider {
  fn provide_layout_and_vertex_in(builder: &mut ShaderRawVertexBuilder, step_mode: VertexStepMode);
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

  pub fn build(self, builder: &mut ShaderRawVertexBuilder, step_mode: VertexStepMode) {
    let layout = ShaderVertexBufferLayout {
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
    vertex_builder: &mut ShaderRawVertexBuilder,
  ) where
    S: SemanticVertexShaderValue<ValueType = Self>;
}

/// Mark self type could use as vertex buffer input
pub trait VertexInShaderNodeType: PrimitiveShaderNodeType {
  fn to_vertex_format() -> VertexFormat;
}

macro_rules! vertex_input_node_impl {
  ($ty: ty, $format: expr) => {
    impl VertexInShaderNodeType for $ty {
      fn to_vertex_format() -> VertexFormat {
        $format
      }
    }
  };
}
vertex_input_node_impl!(f32, VertexFormat::Float32);
vertex_input_node_impl!(Vec2<f32>, VertexFormat::Float32x2);
vertex_input_node_impl!(Vec3<f32>, VertexFormat::Float32x3);
vertex_input_node_impl!(Vec4<f32>, VertexFormat::Float32x4);

vertex_input_node_impl!(u32, VertexFormat::Uint32);
vertex_input_node_impl!(Vec2<u32>, VertexFormat::Uint32x2);
vertex_input_node_impl!(Vec3<u32>, VertexFormat::Uint32x3);
vertex_input_node_impl!(Vec4<u32>, VertexFormat::Uint32x4);

impl<T: VertexInShaderNodeType> VertexInBuilder for T {
  fn build_attribute<S>(
    builder: &mut AttributesListBuilder,
    vertex_builder: &mut ShaderRawVertexBuilder,
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
    vertex_builder: &mut ShaderRawVertexBuilder,
  ) where
    S: SemanticVertexShaderValue<ValueType = Self>,
  {
    let format = Vec4::<f32>::to_vertex_format();

    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 0>>());
    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 1>>());
    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 2>>());
    builder.push(format, vertex_builder.register_vertex_in::<SemanticShaderMat4VertexInColum<S, 3>>());

    let c1 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 0>>();
    let c2 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 1>>();
    let c3 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 2>>();
    let c4 = vertex_builder.query::<SemanticShaderMat4VertexInColum<S, 3>>();

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
