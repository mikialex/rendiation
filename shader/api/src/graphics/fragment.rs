use crate::*;

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderNodeType;
  const NAME: &'static str = std::any::type_name::<Self>();
}

pub struct ShaderFragmentBuilderView<'a> {
  pub(crate) base: &'a mut ShaderFragmentBuilder,
  pub(crate) vertex: &'a mut ShaderVertexBuilder,
}

impl<'a> ShaderFragmentBuilderView<'a> {
  pub fn query_or_interpolate_by<T, V>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
    V: SemanticVertexShaderValue,
    T: SemanticFragmentShaderValue<ValueType = <V as SemanticVertexShaderValue>::ValueType>,
  {
    if let Ok(r) = self.query::<T>() {
      return r;
    }

    set_current_building(ShaderStages::Vertex.into());
    let is_ok = {
      let v_node = self.vertex.query::<V>();
      if let Ok(v_node) = v_node {
        self.vertex.set_vertex_out::<T>(v_node);
        true
      } else {
        false
      }
    };
    set_current_building(None);
    self.vertex.sync_fragment_out(self.base);
    set_current_building(ShaderStages::Fragment.into());

    if is_ok {
      self.query::<T>().unwrap()
    } else {
      self.query_or_insert_default::<T>()
    }
  }
}

impl<'a> Deref for ShaderFragmentBuilderView<'a> {
  type Target = ShaderFragmentBuilder;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}
impl<'a> DerefMut for ShaderFragmentBuilderView<'a> {
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
      ShaderVaryingInterpolation,
      usize,
    ),
  >,

  pub(crate) registry: SemanticRegistry,

  pub frag_output: Vec<(Node<Vec4<f32>>, ColorTargetState)>,
  // improve: check the relationship between depth_output and depth_stencil
  pub depth_stencil: Option<DepthStencilState>,
  // improve: check if all the output should be multisampled target
  pub multisample: MultisampleState,
}

impl ShaderFragmentBuilder {
  pub(crate) fn new() -> Self {
    let mut result = Self {
      fragment_in: Default::default(),
      registry: Default::default(),
      frag_output: Default::default(),
      multisample: Default::default(),
      depth_stencil: Default::default(),
    };

    set_current_building(ShaderStages::Fragment.into());

    let frag_ndc = ShaderInputNode::BuiltIn(ShaderBuiltIn::FragmentNDC).insert_api();
    result.register::<FragmentPosition>(frag_ndc);

    let facing = ShaderInputNode::BuiltIn(ShaderBuiltIn::FragmentFrontFacing).insert_api();
    result.register::<FragmentFrontFacing>(facing);

    let index = ShaderInputNode::BuiltIn(ShaderBuiltIn::FragmentSampleIndex).insert_api();
    result.register::<FragmentSampleIndex>(index);

    let mask = ShaderInputNode::BuiltIn(ShaderBuiltIn::FragmentSampleMask).insert_api();
    result.register::<FragmentSampleMaskInput>(mask);

    set_current_building(None);

    result
  }

  pub fn registry(&self) -> &SemanticRegistry {
    &self.registry
  }

  pub fn discard(&self) {
    call_shader_api(|g| g.discard())
  }

  pub fn query<T: SemanticFragmentShaderValue>(
    &self,
  ) -> Result<Node<T::ValueType>, ShaderBuildError> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    if let Ok(n) = self.registry.query(TypeId::of::<T>(), T::NAME) {
      unsafe { n.cast_type() }
    } else {
      let default: T::ValueType = Default::default();
      self.register::<T>(default)
    }
  }

  pub fn register<T: SemanticFragmentShaderValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) -> Node<T::ValueType> {
    let n = self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
    unsafe { n.cast_type() }
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
    let slot = self.frag_output.len();
    let target = call_shader_api(|g| unsafe { g.define_frag_out(slot).into_node() });
    self.frag_output.push((target, meta.into()));
    slot
  }

  /// always called by material side to provide fragment out
  pub fn set_fragment_out(
    &mut self,
    slot: usize,
    node: impl Into<Node<Vec4<f32>>>,
  ) -> Result<(), ShaderBuildError> {
    let target = self
      .frag_output
      .get_mut(slot)
      .ok_or(ShaderBuildError::FragmentOutputSlotNotDeclared)?
      .0;
    call_shader_api(|g| g.store(node.into().handle(), target.handle()));

    Ok(())
  }

  pub fn get_fragment_out(&mut self, slot: usize) -> Result<Node<Vec4<f32>>, ShaderBuildError> {
    Ok(self.frag_output.get(slot).unwrap().0)
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
