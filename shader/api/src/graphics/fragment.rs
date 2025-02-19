use crate::*;

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderNodeType;
  const NAME: &'static str = std::any::type_name::<Self>();
}

pub struct ShaderFragmentBuilderView<'a> {
  pub(crate) base: &'a mut ShaderFragmentBuilder,
  pub(crate) vertex: &'a mut ShaderVertexBuilder,
}

impl ShaderFragmentBuilderView<'_> {
  pub fn query_or_interpolate_by<T, V>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
    V: SemanticVertexShaderValue,
    V::ValueType: PrimitiveShaderNodeType,
    T: SemanticFragmentShaderValue<ValueType = <V as SemanticVertexShaderValue>::ValueType>,
  {
    if let Some(r) = self.try_query::<T>() {
      return r;
    }

    set_current_building(ShaderStage::Vertex.into());
    let is_ok = {
      let v_node = self.vertex.try_query::<V>();
      if let Some(v_node) = v_node {
        self.vertex.set_vertex_out::<T>(v_node);
        true
      } else {
        false
      }
    };
    set_current_building(None);
    self.vertex.sync_fragment_out(self.base);
    set_current_building(ShaderStage::Fragment.into());

    if is_ok {
      self.query::<T>()
    } else {
      self.error(ShaderBuildError::MissingRequiredDependency(V::NAME));
      self.query_or_insert_default::<T>()
    }
  }
}

impl Deref for ShaderFragmentBuilderView<'_> {
  type Target = ShaderFragmentBuilder;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}
impl DerefMut for ShaderFragmentBuilderView<'_> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.base
  }
}

pub struct ShaderFragmentBuilder {
  // user fragment in
  pub fragment_in: FastHashMap<
    TypeId,
    (
      NodeUntyped,
      PrimitiveShaderValueType,
      ShaderInterpolation,
      usize,
    ),
  >,

  pub(crate) registry: SemanticRegistry,

  pub frag_output: Vec<FragmentOutputPort>,
  // improve: check the relationship between depth_output and depth_stencil
  pub depth_stencil: Option<DepthStencilState>,
  // improve: check if all the output should be multisampled target
  pub multisample: MultisampleState,
  pub(crate) errors: ErrorSink,
}

pub struct FragmentOutputPort {
  node: ShaderNodeRawHandle,
  pub ty: ShaderSizedValueType,
  pub states: ColorTargetState,
}

pub fn is_texture_fmt_blendable(fmt: TextureFormat) -> bool {
  if let Some(ty) = get_suitable_shader_write_ty_from_texture_format(fmt) {
    is_shader_ty_blendable(&ty)
  } else {
    false
  }
}

pub fn is_shader_ty_blendable(ty: &ShaderSizedValueType) -> bool {
  let ty = match ty {
    ShaderSizedValueType::Primitive(p) => p,
    _ => unreachable!(),
  };

  matches!(
    ty,
    PrimitiveShaderValueType::Vec4Float32
      | PrimitiveShaderValueType::Vec3Float32
      | PrimitiveShaderValueType::Vec2Float32
      | PrimitiveShaderValueType::Float32
  )
}

impl FragmentOutputPort {
  pub fn is_blendable(&self) -> bool {
    is_shader_ty_blendable(&self.ty)
  }

  pub fn get_output_var<T: ShaderSizedValueNodeType>(&self) -> ShaderAccessorOf<T> {
    assert_eq!(self.ty, T::sized_ty());
    T::create_accessor_from_raw_ptr(Box::new(self.node))
  }
  pub fn store<T: ShaderSizedValueNodeType>(&self, node: Node<T>) {
    self.get_output_var::<T>().store(node);
  }

  pub fn load<T: ShaderSizedValueNodeType>(&self) -> Node<T> {
    self.get_output_var::<T>().load()
  }
}

impl ShaderFragmentBuilder {
  pub(crate) fn new(errors: ErrorSink) -> Self {
    let mut result = Self {
      fragment_in: Default::default(),
      registry: Default::default(),
      frag_output: Default::default(),
      multisample: Default::default(),
      depth_stencil: Default::default(),
      errors,
    };

    set_current_building(ShaderStage::Fragment.into());

    let frag_ndc = ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::FragPositionIn).insert_api();
    result.register::<FragmentPosition>(frag_ndc);

    let facing = ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::FragFrontFacing).insert_api();
    result.register::<FragmentFrontFacing>(facing);

    let index = ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::FragSampleIndex).insert_api();
    result.register::<FragmentSampleIndex>(index);

    let mask = ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::FragSampleMask).insert_api();
    result.register::<FragmentSampleMaskInput>(mask);

    set_current_building(None);

    result
  }

  pub fn registry(&self) -> &SemanticRegistry {
    &self.registry
  }

  pub fn contains_type_tag<T: Any>(&self) -> bool {
    self.registry.contains_type_tag::<T>()
  }
  pub fn insert_type_tag<T: Any>(&mut self) {
    self.registry.insert_type_tag::<T>()
  }

  pub fn discard(&self) {
    call_shader_api(|g| g.discard())
  }

  pub fn query<T: SemanticFragmentShaderValue>(&self) -> Node<T::ValueType> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
      .unwrap_or_else(|_| unsafe {
        self
          .errors
          .push(ShaderBuildError::MissingRequiredDependency(T::NAME));
        fake_val()
      })
  }

  pub fn try_query<T: SemanticFragmentShaderValue>(&self) -> Option<Node<T::ValueType>> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
      .ok()
  }

  pub fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.query_or_insert_by::<T>(Default::default)
  }

  pub fn query_or_insert_by<T>(&mut self, by: impl FnOnce() -> T::ValueType) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    if let Ok(n) = self.registry.query(TypeId::of::<T>(), T::NAME) {
      unsafe { n.cast_type() }
    } else {
      let default: T::ValueType = by();
      self.register::<T>(default);
      self.query::<T>()
    }
  }

  pub fn register<T: SemanticFragmentShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
  }

  pub fn get_fragment_in<T>(
    &mut self,
  ) -> Result<Node<<T as SemanticVertexShaderValue>::ValueType>, ShaderBuildError>
  where
    T: SemanticFragmentShaderValue,
    T: SemanticVertexShaderValue,
    <T as SemanticVertexShaderValue>::ValueType: PrimitiveShaderNodeType,
    T: SemanticFragmentShaderValue<ValueType = <T as SemanticVertexShaderValue>::ValueType>,
  {
    self
      .fragment_in
      .get(&TypeId::of::<T>())
      .map(|(n, _, _, _)| unsafe { (*n).cast_type() })
      .ok_or(ShaderBuildError::MissingRequiredDependency(
        <T as SemanticVertexShaderValue>::NAME,
      ))
  }

  /// Declare fragment outputs
  pub fn define_out_by(&mut self, meta: impl Into<ColorTargetState>) -> usize {
    let states = meta.into();

    let output_shader_ty = get_suitable_shader_write_ty_from_texture_format(states.format)
      .expect("invalid attachment texture format");

    let slot = self.frag_output.len();
    let node = call_shader_api(|g| g.define_next_frag_out(output_shader_ty.clone()));
    self.frag_output.push(FragmentOutputPort {
      node,
      ty: output_shader_ty,
      states,
    });
    slot
  }

  // this is the mostly common one
  pub fn store_fragment_out_vec4f(&mut self, slot: usize, node: impl Into<Node<Vec4<f32>>>) {
    let node = node.into();
    self.store_fragment_out(slot, node);
  }

  pub fn store_fragment_out<T: ShaderSizedValueNodeType>(&mut self, slot: usize, node: Node<T>) {
    match self.get_fragment_out_var::<T>(slot) {
      Ok(target) => target.store(node),
      Err(err) => {
        self.errors.push(err);
      }
    }
  }

  fn get_fragment_out_var<T: ShaderSizedValueNodeType>(
    &mut self,
    slot: usize,
  ) -> Result<ShaderAccessorOf<T>, ShaderBuildError> {
    Ok(
      self
        .frag_output
        .get(slot)
        .ok_or(ShaderBuildError::FragmentOutputSlotNotDeclared)?
        .get_output_var::<T>(),
    )
  }

  pub fn load_fragment_out<T: ShaderSizedValueNodeType>(
    &mut self,
    slot: usize,
  ) -> Result<Node<T>, ShaderBuildError> {
    Ok(self.get_fragment_out_var::<T>(slot)?.load())
  }

  /// currently we all depend on FragmentDepthOutput in semantic registry to given the final result
  /// this behavior will be changed in future;
  pub fn finalize_depth_write(&mut self) {
    let depth = self.try_query::<FragmentDepthOutput>();
    if let Some(depth) = depth {
      call_shader_api(|api| {
        let target = api.define_frag_depth_output();
        api.store(depth.handle(), target)
      });
    }
  }
  pub fn error(&mut self, err: ShaderBuildError) {
    self.errors.push(err);
  }
}

pub struct ColorTargetStateBuilder {
  state: ColorTargetState,
}

impl ColorTargetStateBuilder {
  pub fn with_blend(mut self, blend: impl Into<BlendState>) -> Self {
    self.state.blend = Some(blend.into());
    self
  }
  pub fn with_alpha_blend(mut self) -> Self {
    self.state.blend = Some(BlendState::ALPHA_BLENDING);
    self
  }
}

pub fn channel(format: TextureFormat) -> ColorTargetStateBuilder {
  ColorTargetStateBuilder {
    state: ColorTargetState {
      format,
      blend: None,
      write_mask: ColorWrites::ALL,
    },
  }
}

impl From<ColorTargetStateBuilder> for ColorTargetState {
  fn from(b: ColorTargetStateBuilder) -> Self {
    b.state
  }
}

/// maybe we should let user decide the output type if the texture format can be written by different shader ty.
pub fn get_suitable_shader_write_ty_from_texture_format(
  format: TextureFormat,
) -> Option<ShaderSizedValueType> {
  let ty = match format {
    TextureFormat::R8Unorm => PrimitiveShaderValueType::Float32,
    TextureFormat::R8Snorm => PrimitiveShaderValueType::Float32,
    TextureFormat::R8Uint => PrimitiveShaderValueType::Uint32,
    TextureFormat::R8Sint => PrimitiveShaderValueType::Int32,
    TextureFormat::R16Uint => PrimitiveShaderValueType::Uint32,
    TextureFormat::R16Sint => PrimitiveShaderValueType::Int32,
    TextureFormat::R16Unorm => PrimitiveShaderValueType::Float32,
    TextureFormat::R16Snorm => PrimitiveShaderValueType::Float32,
    TextureFormat::R16Float => PrimitiveShaderValueType::Float32,
    TextureFormat::Rg8Unorm => PrimitiveShaderValueType::Vec2Float32,
    TextureFormat::Rg8Snorm => PrimitiveShaderValueType::Vec2Float32,
    TextureFormat::Rg8Uint => PrimitiveShaderValueType::Vec2Uint32,
    TextureFormat::Rg8Sint => PrimitiveShaderValueType::Vec2Int32,
    TextureFormat::R32Uint => PrimitiveShaderValueType::Uint32,
    TextureFormat::R32Sint => PrimitiveShaderValueType::Int32,
    TextureFormat::R32Float => PrimitiveShaderValueType::Float32,
    TextureFormat::Rg16Uint => PrimitiveShaderValueType::Vec2Uint32,
    TextureFormat::Rg16Sint => PrimitiveShaderValueType::Vec2Int32,
    TextureFormat::Rg16Unorm => PrimitiveShaderValueType::Vec2Float32,
    TextureFormat::Rg16Snorm => PrimitiveShaderValueType::Vec2Float32,
    TextureFormat::Rg16Float => PrimitiveShaderValueType::Vec2Float32,
    TextureFormat::Rgba8Unorm => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgba8UnormSrgb => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgba8Snorm => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgba8Uint => PrimitiveShaderValueType::Vec4Uint32,
    TextureFormat::Rgba8Sint => PrimitiveShaderValueType::Vec4Int32,
    TextureFormat::Bgra8Unorm => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Bgra8UnormSrgb => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgb9e5Ufloat => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgb10a2Uint => PrimitiveShaderValueType::Vec4Uint32,
    TextureFormat::Rgb10a2Unorm => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rg11b10Ufloat => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::R64Uint => return None,
    TextureFormat::Rg32Uint => PrimitiveShaderValueType::Vec2Uint32,
    TextureFormat::Rg32Sint => PrimitiveShaderValueType::Vec2Int32,
    TextureFormat::Rg32Float => PrimitiveShaderValueType::Vec2Float32,
    TextureFormat::Rgba16Uint => PrimitiveShaderValueType::Vec4Uint32,
    TextureFormat::Rgba16Sint => PrimitiveShaderValueType::Vec4Int32,
    TextureFormat::Rgba16Unorm => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgba16Snorm => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgba16Float => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Rgba32Uint => PrimitiveShaderValueType::Uint32,
    TextureFormat::Rgba32Sint => PrimitiveShaderValueType::Int32,
    TextureFormat::Rgba32Float => PrimitiveShaderValueType::Vec4Float32,
    TextureFormat::Stencil8 => PrimitiveShaderValueType::Uint32,
    TextureFormat::Depth16Unorm => PrimitiveShaderValueType::Float32,
    TextureFormat::Depth24Plus => PrimitiveShaderValueType::Float32,
    TextureFormat::Depth24PlusStencil8 => PrimitiveShaderValueType::Float32,
    TextureFormat::Depth32Float => PrimitiveShaderValueType::Float32,
    TextureFormat::Depth32FloatStencil8 => PrimitiveShaderValueType::Float32,
    _ => return None,
  };

  ShaderSizedValueType::Primitive(ty).into()
}
