use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use crate::*;

struct PendingResolve {
  unresolved: Rc<Cell<ShaderGraphNodeRawHandle>>,
  depend_by: Vec<ShaderGraphNodeRawHandle>,
}

#[derive(Default)]
pub struct SemanticRegistry {
  registered: HashMap<TypeId, NodeUntyped>,
  pending_resolve: HashMap<TypeId, PendingResolve>,
}

impl SemanticRegistry {
  pub fn query_last(&mut self, id: TypeId) -> NodeUntyped {
    let cell = &self
      .pending_resolve
      .entry(id)
      .or_insert_with(|| todo!())
      .unresolved;
    let cell: &Rc<Cell<ShaderGraphNodeRawHandle>> = unsafe { std::mem::transmute(cell) };
    Node {
      handle: NodeInner::Unresolved(cell.clone()),
      phantom: PhantomData,
    }
  }

  pub fn resolve_all_pending(&mut self, root_scope: &mut ShaderGraphScope) {
    let registered = &self.registered;
    self.pending_resolve.drain().for_each(|(id, to_resolve)| {
      if let Some(target) = registered.get(&id) {
        to_resolve.unresolved.set(target.handle());
        to_resolve.depend_by.iter().for_each(|dep| {
          root_scope
            .nodes
            .connect_node(target.handle().handle, dep.handle)
        })
      }
    })
  }

  pub fn query(&mut self, id: TypeId) -> Result<NodeUntyped, ShaderGraphBuildError> {
    self
      .registered
      .get(&id)
      .map(|node| {
        let n: &Node<Mutable<AnyType>> = unsafe { std::mem::transmute(node) };
        n.get()
      })
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency)
  }

  pub fn register(&mut self, id: TypeId, node: NodeUntyped) {
    self.registered.entry(id).or_insert_with(|| node);
  }
}
