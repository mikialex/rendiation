use std::any::TypeId;

use crate::*;

pub struct ShaderGraphBindGroupBuilder {
  pub(crate) current_stage: ShaderStages,
  pub bindings: Vec<ShaderGraphBindGroup>,
}

impl Default for ShaderGraphBindGroupBuilder {
  fn default() -> Self {
    Self {
      current_stage: ShaderStages::Vertex,
      bindings: vec![Default::default(); 5],
    }
  }
}

impl ShaderGraphBindGroupBuilder {
  #[inline(never)]
  fn register_uniform_inner(
    &mut self,
    type_id: TypeId,
    bindgroup_index: usize,
    ty: ShaderValueType,
  ) -> NodeUntyped {
    if let Ok(node) = self.query_uniform_inner(type_id, bindgroup_index) {
      return node;
    }

    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let node = ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    }
    .insert_graph();

    let (node_vertex, node_fragment) = match self.current_stage {
      ShaderStages::Vertex => (node.handle().into(), None),
      ShaderStages::Fragment => (None, node.handle().into()),
    };

    bindgroup.bindings.push((
      ShaderGraphBindEntry {
        ty,
        node_vertex,
        node_fragment,
      },
      type_id,
    ));

    node
  }

  #[inline(never)]
  fn query_uniform_inner(
    &mut self,
    type_id: TypeId,
    bindgroup_index: usize,
  ) -> Result<NodeUntyped, ShaderGraphBuildError> {
    let current_stage = self.current_stage;
    let bindgroup = &mut self.bindings[bindgroup_index];

    bindgroup
      .bindings
      .iter_mut()
      .enumerate()
      .find(|(_, entry)| entry.1 == type_id)
      .map(|(i, (entry, _))| unsafe {
        let node = match current_stage {
          ShaderStages::Vertex => &mut entry.node_vertex,
          ShaderStages::Fragment => &mut entry.node_fragment,
        };
        node
          .get_or_insert_with(|| {
            ShaderGraphInputNode::Uniform {
              bindgroup_index,
              entry_index: i,
            }
            .insert_graph::<AnyType>()
            .handle()
          })
          .into_node()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  #[inline]
  pub fn register_uniform<T: SemanticShaderUniform>(&mut self) -> Node<T::Node> {
    let node = self.register_uniform_inner(
      TypeId::of::<T>(),
      T::TYPE.binding_index(),
      T::Node::to_type(),
    );
    unsafe { node.cast_type() }
  }

  #[inline]
  pub fn register_uniform_by<T: SemanticShaderUniform>(&mut self, _instance: &T) -> Node<T::Node> {
    self.register_uniform::<T>()
  }

  #[inline]
  pub fn query_uniform<T: SemanticShaderUniform>(
    &mut self,
  ) -> Result<Node<T::Node>, ShaderGraphBuildError> {
    let result = self.query_uniform_inner(TypeId::of::<T>(), T::TYPE.binding_index());
    result.map(|n| unsafe { n.cast_type() })
  }
}
