use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

use crate::*;

pub trait SemanticFragmentShaderValue: Any {
  type ValueType: ShaderGraphNodeType;
  const NAME: &'static str = "unnamed";
}

pub struct ShaderGraphFragmentBuilder {
  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

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

impl std::ops::Deref for ShaderGraphFragmentBuilder {
  type Target = ShaderGraphBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

impl std::ops::DerefMut for ShaderGraphFragmentBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bindgroups
  }
}

impl ShaderGraphFragmentBuilder {
  pub fn create(mut vertex: ShaderGraphVertexBuilder) -> Self {
    let builder = ShaderGraphBuilder::default();
    set_build_graph(builder);

    let mut fragment_in = HashMap::default();
    vertex.vertex_out.iter().for_each(|(id, (_, ty, index))| {
      let node = ShaderGraphNodeData::Input(ShaderGraphInputNode::FragmentIn {
        ty: *ty,
        index: *index,
      })
      .insert_graph();
      fragment_in.insert(
        *id,
        (node, *ty, ShaderVaryingInterpolation::Perspective, *index),
      );
    });
    // todo setup fragin into registry

    vertex.current_stage = ShaderStages::Fragment;

    Self {
      bindgroups: vertex.bindgroups,
      fragment_in,
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

  pub fn get_fragment_in<T>(&mut self) -> Result<Node<T::ValueType>, ShaderGraphBuildError>
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderGraphNodeType,
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

  pub fn extract(&self) -> ShaderGraphBuilder {
    take_build_graph()
  }
}
