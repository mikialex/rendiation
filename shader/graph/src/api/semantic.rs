use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use crate::*;

#[derive(Default)]
pub struct SemanticRegistry {
  registered: HashMap<TypeId, NodeUntyped>,
  pending_resolve: HashMap<TypeId, PendingResolve>,
}

impl SemanticRegistry {
  pub fn query(&mut self, id: TypeId) -> Result<&Node<Mutable<AnyType>>, ShaderGraphBuildError> {
    self
      .registered
      .get(&id)
      .map(|node| unsafe { std::mem::transmute(node) })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn register(&mut self, id: TypeId, node: NodeUntyped) {
    self.registered.entry(id).or_insert_with(|| node);
  }
}
