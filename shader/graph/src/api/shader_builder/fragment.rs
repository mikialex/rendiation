use wgpu_types::{BlendState, ColorWrites, TextureFormat};

use crate::*;

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = core::intrinsics::type_name::<Self>();
}

pub struct ShaderGraphFragmentBuilder {
  // user fragment in
  pub fragment_in: HashMap<
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
  pub depth_output: Option<Node<f32>>,
  // improve: check the relationship between depth_output and depth_stencil
  pub depth_stencil: Option<DepthStencilState>,
  // improve: check if all the output should be multisampled target
  pub multisample: MultisampleState,
}

impl ShaderGraphFragmentBuilder {
  pub(crate) fn new() -> Self {
    set_current_building(ShaderStages::Fragment.into());

    // todo setup builtin fragment in

    set_current_building(None);

    Self {
      fragment_in: Default::default(),
      registry: Default::default(),
      frag_output: Default::default(),
      multisample: Default::default(),
      depth_output: None,
      depth_stencil: Default::default(),
    }
  }

  pub fn discard(&self) {
    ShaderSideEffectNode::Termination.insert_graph_bottom();
  }

  pub fn query<T: SemanticFragmentShaderValue>(
    &self,
  ) -> Result<&NodeMutable<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn query_or_insert_default<T>(&mut self) -> &NodeMutable<T::ValueType>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
  {
    if let Ok(n) = self.registry.query(TypeId::of::<T>(), T::NAME) {
      unsafe { std::mem::transmute(n) }
    } else {
      let default: T::ValueType = Default::default();
      self.register::<T>(default)
    }
  }

  pub fn register<T: SemanticFragmentShaderValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) -> &NodeMutable<T::ValueType> {
    let n = self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
    unsafe { std::mem::transmute(n) }
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
  pub fn out_by(&mut self, meta: impl Into<ColorTargetState>) -> usize {
    self.frag_output.push((consts(Vec4::zero()), meta.into()));
    self.frag_output.len() - 1
  }

  /// always called by material side to provide fragment out
  pub fn set_fragment_out(
    &mut self,
    slot: usize,
    node: impl Into<Node<Vec4<f32>>>,
  ) -> Result<(), ShaderGraphBuildError> {
    self
      .frag_output
      .get_mut(slot)
      .ok_or(ShaderGraphBuildError::FragmentOutputSlotNotDeclared)?
      .0 = node.into();
    Ok(())
  }

  pub fn set_explicit_depth(&mut self, node: Node<f32>) {
    self.depth_output = node.into()
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
