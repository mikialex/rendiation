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
    if let Some(r) = self.try_query::<T>() {
      return r;
    }

    set_current_building(ShaderStages::Vertex.into());
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
    set_current_building(ShaderStages::Fragment.into());

    if is_ok {
      self.query::<T>()
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

  pub frag_output: Vec<(LocalVarNode<Vec4<f32>>, ColorTargetState)>,
  // improve: check the relationship between depth_output and depth_stencil
  pub depth_stencil: Option<DepthStencilState>,
  // improve: check if all the output should be multisampled target
  pub multisample: MultisampleState,
  pub(crate) errors: ErrorSink,
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

    set_current_building(ShaderStages::Fragment.into());

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
    let slot = self.frag_output.len();
    let target = call_shader_api(|g| unsafe { g.define_next_frag_out().into_node() });
    self.frag_output.push((target, meta.into()));
    slot
  }

  /// always called by material side to provide fragment out
  pub fn store_fragment_out(&mut self, slot: usize, node: impl Into<Node<Vec4<f32>>>) {
    match self.get_fragment_out_var(slot) {
      Ok(target) => target.store(node.into()),
      Err(err) => {
        self.errors.push(err);
      }
    }
  }

  fn get_fragment_out_var(
    &mut self,
    slot: usize,
  ) -> Result<LocalVarNode<Vec4<f32>>, ShaderBuildError> {
    Ok(
      self
        .frag_output
        .get(slot)
        .ok_or(ShaderBuildError::FragmentOutputSlotNotDeclared)?
        .0,
    )
  }

  pub fn load_fragment_out(&mut self, slot: usize) -> Result<Node<Vec4<f32>>, ShaderBuildError> {
    Ok(self.get_fragment_out_var(slot)?.load())
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
