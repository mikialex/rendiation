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

pub struct ShaderVertexBuilder {
  // user vertex in
  pub vertex_in: FastHashMap<TypeId, VertexIOInfo>,
  pub vertex_layouts: Vec<ShaderVertexBufferLayout>,
  pub primitive_state: PrimitiveState,

  // user semantic vertex
  pub(crate) registry: SemanticRegistry,

  // user vertex out
  pub vertex_out: FastHashMap<TypeId, (VertexIOInfo, ShaderInterpolation)>,
  pub(crate) vertex_out_not_synced_to_fragment: FastHashSet<TypeId>,
  pub(crate) errors: ErrorSink,
}

#[derive(Copy, Clone)]
pub struct VertexIOInfo {
  pub node: ShaderNodeRawHandle,
  pub ty: PrimitiveShaderValueType,
  pub location: usize,
}

impl ShaderVertexBuilder {
  pub(crate) fn new(errors: ErrorSink) -> Self {
    Self {
      vertex_in: Default::default(),
      registry: Default::default(),
      vertex_out: Default::default(),
      vertex_layouts: Default::default(),
      primitive_state: PrimitiveState {
        cull_mode: Some(Face::Back),
        ..Default::default()
      },
      vertex_out_not_synced_to_fragment: Default::default(),
      errors,
    }
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

  pub fn registry(&self) -> &SemanticRegistry {
    &self.registry
  }

  pub fn query<T: SemanticVertexShaderValue>(&mut self) -> Node<T::ValueType> {
    self.try_query::<T>().unwrap_or_else(|| unsafe {
      self
        .errors
        .push(ShaderBuildError::MissingRequiredDependency(T::NAME));
      fake_val()
    })
  }

  pub fn try_query<T: SemanticVertexShaderValue>(&mut self) -> Option<Node<T::ValueType>> {
    if self.registry.try_query_vertex_stage::<T>().is_err() {
      let id = TypeId::of::<T>();
      if id == TypeId::of::<VertexIndex>() {
        let vertex_index =
          ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::VertexIndex).insert_api();
        self.register::<VertexIndex>(vertex_index);
      }

      if id == TypeId::of::<VertexInstanceIndex>() {
        let instance_index =
          ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::VertexInstanceIndex).insert_api();
        self.register::<VertexInstanceIndex>(instance_index);
      }
    }

    self.registry.try_query_vertex_stage::<T>().ok()
  }

  pub fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.query_or_insert_by::<T>(Default::default)
  }

  pub fn query_or_insert_by<T>(&mut self, by: impl FnOnce() -> T::ValueType) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    if let Some(n) = self.try_query::<T>() {
      n
    } else {
      let default: T::ValueType = by();
      self.register::<T>(default);
      self.query::<T>()
    }
  }

  pub fn register<T: SemanticVertexShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self.registry.register_vertex_stage::<T>(node);
  }

  /// return registered location
  pub fn register_vertex_in<T>(&mut self) -> u32
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.register_vertex_in_inner(T::ValueType::PRIMITIVE_TYPE, TypeId::of::<T>())
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

  /// default using perspective interpolation
  pub fn set_vertex_out<T>(&mut self, node: impl Into<Node<T::ValueType>>)
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.set_vertex_out_with_given_interpolate::<T>(node, ShaderInterpolation::Perspective)
  }

  pub fn set_vertex_out_with_given_interpolate<T>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
    mut interpolation: ShaderInterpolation,
  ) where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    let location = self.vertex_out.len();
    let id = TypeId::of::<T>();
    let target = self
      .vertex_out
      .entry(id)
      .or_insert_with(|| {
        let ty = T::ValueType::PRIMITIVE_TYPE;
        if !ty.vertex_out_could_interpolated() {
          interpolation = ShaderInterpolation::Flat
        }
        let node = call_shader_api(|api| api.define_next_vertex_output(ty, Some(interpolation)));

        (VertexIOInfo { node, ty, location }, interpolation)
      })
      .0
      .node;
    call_shader_api(|api| api.store(node.into().handle(), target));

    self.vertex_out_not_synced_to_fragment.insert(id);
  }

  pub fn register_vertex<V>(&mut self, step_mode: VertexStepMode)
  where
    V: ShaderVertexInProvider,
  {
    V::provide_layout_and_vertex_in(self, step_mode)
  }

  /// currently we all depend on ClipPosition in semantic registry to given the final result
  /// this behavior will be changed in future;
  pub fn finalize_position_write(&mut self) {
    let position = self.query_or_insert_default::<ClipPosition>();
    call_shader_api(|api| {
      let target = api.define_vertex_position_output();
      api.store(position.handle(), target)
    });
  }

  pub fn error(&mut self, err: ShaderBuildError) {
    self.errors.push(err);
  }
}

pub trait ShaderVertexInProvider {
  fn provide_layout_and_vertex_in(builder: &mut ShaderVertexBuilder, step_mode: VertexStepMode);
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

  pub fn build(self, builder: &mut ShaderVertexBuilder, step_mode: VertexStepMode) {
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
    vertex_builder: &mut ShaderVertexBuilder,
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
    vertex_builder: &mut ShaderVertexBuilder,
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
    vertex_builder: &mut ShaderVertexBuilder,
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
