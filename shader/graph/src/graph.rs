use std::{any::TypeId, cell::RefCell, collections::HashMap, marker::PhantomData};

use arena_graph::ArenaGraph;

use crate::*;

pub enum NodeInner {
  Settled(ShaderGraphNodeRawHandle),
  Unresolved(Rc<PendingResolve>),
}

pub struct PendingResolve {
  pub current: Cell<ShaderGraphNodeRawHandle>,
  pub last_depends_history: RefCell<Vec<ShaderGraphNodeRawHandle>>,
}

#[repr(transparent)]
pub struct Node<T> {
  pub(crate) phantom: PhantomData<T>,
  pub(crate) handle: NodeInner,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderGraphNodeRawHandle {
    match &self.handle {
      NodeInner::Settled(h) => *h,
      NodeInner::Unresolved(v) => v.current.get(),
    }
  }
}

impl<T> From<T> for Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  fn from(input: T) -> Self {
    ShaderGraphNodeExpr::Const(ConstNode {
      data: input.to_primitive(),
    })
    .insert_graph()
  }
}

// this for not include samplers/textures as attributes
pub trait ShaderGraphAttributeNodeType: ShaderGraphNodeType {}

#[derive(Copy, Clone)]
pub struct AnyType;

impl<T> Node<T> {
  pub fn cast_untyped_node(&self) -> NodeUntyped {
    unsafe { std::mem::transmute_copy(self) }
  }
}

// impl<T: ShaderGraphNodeType> ShaderGraphNode<T> {
//   #[must_use]
//   pub fn new(data: ShaderGraphNodeData) -> Self {
//     Self {
//       data,
//       phantom: PhantomData,
//     }
//   }

//   #[must_use]
//   pub fn into_any(self) -> ShaderGraphNodeUntyped {
//     unsafe { std::mem::transmute(self) }
//   }

//   #[must_use]
//   pub fn into_typed(self) -> ShaderGraphNode<T> {
//     unsafe { std::mem::transmute(self) }
//   }

//   pub fn unwrap_as_input(&self) -> &ShaderGraphInputNode {
//     match &self.data {
//       ShaderGraphNodeData::Input(n) => n,
//       _ => panic!("unwrap as input failed"),
//     }
//   }
// }

pub type NodeUntyped = Node<AnyType>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderGraphNodeRawHandle {
  pub(crate) handle: ArenaGraphNodeHandle<ShaderGraphNodeData>,
  pub(crate) graph_id: usize,
}

impl ShaderGraphNodeRawHandle {
  /// # Safety
  ///
  /// force type casting
  pub unsafe fn into_node<X>(&self) -> Node<X> {
    Node {
      handle: NodeInner::Settled(*self),
      phantom: PhantomData,
    }
  }

  pub fn into_node_untyped(&self) -> NodeUntyped {
    unsafe { self.into_node() }
  }
}

pub struct ShaderGraphBuilder {
  scope_count: usize,
  pub scopes: Vec<ShaderGraphScope>,
  pub struct_defines: HashMap<TypeId, &'static ShaderStructMetaInfo>,
}

impl Default for ShaderGraphBuilder {
  fn default() -> Self {
    Self {
      scope_count: 0,
      scopes: vec![ShaderGraphScope::new(0)],
      struct_defines: Default::default(),
    }
  }
}

impl ShaderGraphBuilder {
  pub fn top_scope_mut(&mut self) -> &mut ShaderGraphScope {
    self.scopes.last_mut().unwrap()
  }
  pub fn top_scope(&self) -> &ShaderGraphScope {
    self.scopes.last().unwrap()
  }

  pub fn push_scope(&mut self) -> &mut ShaderGraphScope {
    self.scope_count += 1;
    self.scopes.push(ShaderGraphScope::new(self.scope_count));
    self.top_scope_mut()
  }

  pub fn pop_scope(&mut self) -> ShaderGraphScope {
    let mut scope = self.scopes.pop().unwrap();
    scope.resolve_all_pending();
    scope
  }
}

pub struct ShaderGraphScope {
  pub graph_guid: usize,
  pub has_side_effect: bool,
  pub nodes: ArenaGraph<ShaderGraphNodeData>,
  /// every node write in this scope in sequence
  pub inserted: Vec<ShaderGraphNodeRawHandle>,
  /// every effect node has wrote to this scope
  ///
  /// To keep the control flow order correct:
  /// any effect node write should depend any inserted before,
  /// any node write should depend all written effect nodes ;
  pub barriers: Vec<ShaderGraphNodeRawHandle>,
  /// any scoped inserted nodes's dependency which not exist in current scope.
  /// when scope popped, ths captured node will generate dependency in parent scope
  /// and pop again if the node is not find in parent scope
  pub captured: Vec<ShaderGraphNodeRawHandle>,
  /// any scoped inserted nodes's write to dependency which not exist in current scope.
  /// when scope popped, ditto
  ///
  /// require clone Rc<PendingResolve> is to add the implicit write node after the scope
  pub writes: Vec<(Rc<PendingResolve>, ShaderGraphNodeRawHandle)>,

  pub unresolved: Vec<Rc<PendingResolve>>,
}

impl ShaderGraphScope {
  pub fn new(graph_guid: usize) -> Self {
    Self {
      graph_guid,
      has_side_effect: false,
      nodes: Default::default(),
      inserted: Default::default(),
      barriers: Default::default(),
      captured: Default::default(),
      writes: Default::default(),
      unresolved: Default::default(),
    }
  }

  pub fn resolve_all_pending(&mut self) {
    let nodes = &mut self.nodes;
    self.unresolved.drain(..).for_each(|p| {
      p.last_depends_history.borrow().iter().for_each(|old_h| {
        let old = nodes.get_node(old_h.handle);
        let mut dependency = old.from().clone();
        dependency.drain(..).for_each(|d| {
          let dd = nodes.get_node_mut(d);
          dd.data_mut().replace_dependency(*old_h, p.current.get());
          nodes.connect_node(p.current.get().handle, d);
          // todo cut old connection;
          // todo check fix duplicate connection
        })
      })
    })
  }

  pub fn insert_node(&mut self, node: ShaderGraphNodeData) -> NodeUntyped {
    let handle = ShaderGraphNodeRawHandle {
      handle: self.nodes.create_node(node),
      graph_id: self.graph_guid,
    };
    self.inserted.push(handle);
    handle.into_node_untyped()
  }
}
