use crate::*;

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
}

pub struct ShaderGraphFragmentBuilder {
  // user fragment in
  pub(crate) fragment_in: HashMap<
    TypeId,
    (
      NodeUntyped,
      PrimitiveShaderValueType,
      ShaderVaryingInterpolation,
      usize,
    ),
  >,

  registry: SemanticRegistry,

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
    &mut self,
  ) -> Result<&NodeMutable<T::ValueType>, ShaderGraphBuildError> {
    self
      .registry
      .query(TypeId::of::<T>())
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn register<T: SemanticFragmentShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    self
      .registry
      .register(TypeId::of::<T>(), node.into().cast_untyped_node());
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
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  /// always called by pass side to declare outputs
  pub fn push_fragment_out_slot(&mut self, meta: ColorTargetState) {
    self.frag_output.push((consts(Vec4::zero()), meta));
  }

  /// always called by material side to provide fragment out
  pub fn set_fragment_out(
    &mut self,
    slot: usize,
    node: Node<Vec4<f32>>,
  ) -> Result<(), ShaderGraphBuildError> {
    self
      .frag_output
      .get_mut(slot)
      .ok_or(ShaderGraphBuildError::FragmentOutputSlotNotDeclared)?
      .0 = node;
    Ok(())
  }

  pub fn set_explicit_depth(&mut self, node: Node<f32>) {
    self.depth_output = node.into()
  }
}
