use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use arena_graph::ArenaGraph;

use crate::*;

#[derive(Clone)]
pub struct Node<T> {
  pub(crate) handle: Rc<Cell<ShaderGraphNodeRawHandle<T>>>,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderGraphNodeRawHandle<T> {
    self.handle.get()
  }
  pub fn clone_inner(&self) -> Rc<Cell<ShaderGraphNodeRawHandleUntyped>> {
    unsafe { std::mem::transmute(self.handle.clone()) }
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
  /// cast the underlayer handle to untyped, this cast is safe because
  /// we consider this a kind of up casting. Use this will reduce the
  /// unsafe code when we create ShaderGraphNodeData
  pub fn cast_untyped(&self) -> ShaderGraphNodeRawHandleUntyped {
    unsafe { self.handle.get().cast_type() }
  }

  pub fn cast_untyped_node(&self) -> NodeUntyped {
    self.cast_untyped().into()
  }
}

pub struct ShaderGraphNode<T> {
  phantom: PhantomData<T>,
  pub data: ShaderGraphNodeData,
}

impl<T: ShaderGraphNodeType> ShaderGraphNode<T> {
  #[must_use]
  pub fn new(data: ShaderGraphNodeData) -> Self {
    Self {
      data,
      phantom: PhantomData,
    }
  }

  #[must_use]
  pub fn into_any(self) -> ShaderGraphNodeUntyped {
    unsafe { std::mem::transmute(self) }
  }

  #[must_use]
  pub fn into_typed(self) -> ShaderGraphNode<T> {
    unsafe { std::mem::transmute(self) }
  }

  pub fn unwrap_as_input(&self) -> &ShaderGraphInputNode {
    match &self.data {
      ShaderGraphNodeData::Input(n) => n,
      _ => panic!("unwrap as input failed"),
    }
  }
}

impl<T: ShaderGraphNodeType> From<ShaderGraphNodeRawHandle<T>> for Node<T> {
  fn from(handle: ShaderGraphNodeRawHandle<T>) -> Self {
    Node {
      handle: Rc::new(Cell::new(handle)),
    }
  }
}

pub type NodeUntyped = Node<AnyType>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub struct ShaderGraphNodeRawHandle<T> {
  pub(crate) handle: ArenaGraphNodeHandle<ShaderGraphNode<T>>,
  pub(crate) graph_id: usize,
}

impl<T> ShaderGraphNodeRawHandle<T> {
  /// # Safety
  ///
  /// force type casting
  pub unsafe fn cast_type<X>(&self) -> ShaderGraphNodeRawHandle<X> {
    let t: &ShaderGraphNodeRawHandle<X> = std::mem::transmute(self);
    *t
  }

  pub fn cast_untyped(&self) -> ShaderGraphNodeRawHandleUntyped {
    unsafe { self.cast_type() }
  }
}

impl<T> Clone for ShaderGraphNodeRawHandle<T> {
  fn clone(&self) -> ShaderGraphNodeRawHandle<T> {
    Self {
      handle: self.handle,
      graph_id: self.graph_id,
    }
  }
}

impl<T> Copy for ShaderGraphNodeRawHandle<T> {}

impl<T> PartialEq for ShaderGraphNodeRawHandle<T> {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle && self.graph_id == other.graph_id
  }
}

impl<T> Eq for ShaderGraphNodeRawHandle<T> {}

use core::hash::Hash;
impl<T> Hash for ShaderGraphNodeRawHandle<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.handle.hash(state);
  }
}

pub type ShaderGraphNodeRawHandleUntyped = ShaderGraphNodeRawHandle<AnyType>;

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
    self.scopes.pop().unwrap()
  }
}

pub struct ShaderGraphScope {
  pub graph_guid: usize,
  pub has_side_effect: bool,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub inserted: Vec<ShaderGraphNodeRawHandleUntyped>,
  pub barriers: Vec<ShaderGraphNodeRawHandleUntyped>,
  pub captured: Vec<ShaderGraphNodeRawHandleUntyped>,
  pub writes: Vec<(
    Rc<Cell<ShaderGraphNodeRawHandleUntyped>>,
    ShaderGraphNodeRawHandleUntyped,
  )>,
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
    }
  }

  pub fn insert_node<T: ShaderGraphNodeType>(&mut self, node: ShaderGraphNode<T>) -> NodeUntyped {
    let handle = ShaderGraphNodeRawHandle {
      handle: self.nodes.create_node(node.into_any()),
      graph_id: self.graph_guid,
    };
    self.inserted.push(handle);
    handle.into()
  }
}
