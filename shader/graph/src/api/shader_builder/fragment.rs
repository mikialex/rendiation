use crate::*;

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = std::any::type_name::<Self>();
}

pub struct ShaderGraphFragmentBuilderView<'a> {
  pub(crate) base: &'a mut ShaderGraphFragmentBuilder,
  pub(crate) vertex: &'a mut ShaderGraphVertexBuilder,
}

impl<'a> ShaderGraphFragmentBuilderView<'a> {
  pub fn query_or_interpolate_by<T, V>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
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

impl<'a> Deref for ShaderGraphFragmentBuilderView<'a> {
  type Target = ShaderGraphFragmentBuilder;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}
impl<'a> DerefMut for ShaderGraphFragmentBuilderView<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.base
  }
}

pub struct ShaderGraphFragmentBuilder {
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

impl ShaderGraphFragmentBuilder {
  pub(crate) fn new() -> Self {
    let mut result = Self {
      fragment_in: Default::default(),
      registry: Default::default(),
      frag_output: Default::default(),
      multisample: Default::default(),
      depth_stencil: Default::default(),
    };

    set_current_building(ShaderStages::Fragment.into());

    let frag_ndc = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::FragmentNDC).insert_graph();
    result.register::<FragmentPosition>(frag_ndc);

    let facing = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::FragmentFrontFacing).insert_graph();
    result.register::<FragmentFrontFacing>(facing);

    let index = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::FragmentSampleIndex).insert_graph();
    result.register::<FragmentSampleIndex>(index);

    let mask = ShaderGraphInputNode::BuiltIn(ShaderBuiltIn::FragmentSampleMask).insert_graph();
    result.register::<FragmentSampleMaskInput>(mask);

    set_current_building(None);

    result
  }

  pub fn registry(&self) -> &SemanticRegistry {
    &self.registry
  }

  pub fn discard(&self) {
    ShaderSideEffectNode::Termination.insert_graph_bottom();
  }

  pub fn query<T: SemanticFragmentShaderValue>(
    &self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
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
  ) -> Result<Node<<T as SemanticVertexShaderValue>::ValueType>, ShaderGraphBuildError>
  where
    T: SemanticFragmentShaderValue,
    T: SemanticVertexShaderValue,
    <T as SemanticVertexShaderValue>::ValueType: PrimitiveShaderGraphNodeType,
    T: SemanticFragmentShaderValue<ValueType = <T as SemanticVertexShaderValue>::ValueType>,
  {
    self
      .fragment_in
      .get(&TypeId::of::<T>())
      .map(|(n, _, _, _)| unsafe { (*n).cast_type() })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency(
        <T as SemanticVertexShaderValue>::NAME,
      ))
  }

  /// Declare fragment outputs
  pub fn define_out_by(&mut self, meta: impl Into<ColorTargetState>) -> usize {
    self.frag_output.push((consts(Vec4::zero()), meta.into()));
    self.frag_output.len() - 1
  }

  /// always called by material side to provide fragment out
  pub fn set_fragment_out(
    &mut self,
    slot: usize,
    node: impl Into<Node<Vec4<f32>>>,
  ) -> Result<(), ShaderGraphBuildError> {
    // because discard has side effect, we have to use a write to get correct dependency
    let write = ShaderGraphNode::Write {
      new: node.into().handle(),
      old: None,
    }
    .insert_graph();

    self
      .frag_output
      .get_mut(slot)
      .ok_or(ShaderGraphBuildError::FragmentOutputSlotNotDeclared)?
      .0 = write;
    Ok(())
  }

  pub fn get_fragment_out(
    &mut self,
    slot: usize,
  ) -> Result<Node<Vec4<f32>>, ShaderGraphBuildError> {
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
