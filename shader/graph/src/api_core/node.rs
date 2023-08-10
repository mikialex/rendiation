use crate::*;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Node<T> {
  pub(crate) phantom: PhantomData<T>,
  pub(crate) handle: ShaderGraphNodeRawHandle,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderGraphNodeRawHandle {
    self.handle
  }

  /// # Safety
  /// force type casting
  pub unsafe fn cast_type<X>(self) -> Node<X>
  where
    X: ShaderGraphNodeType,
  {
    modify_graph(|g| g.register_ty(X::TYPE));
    std::mem::transmute(self)
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

pub struct NodeMutable<T> {
  pub(crate) inner: Node<T>,
}

impl<T: ShaderGraphNodeType> Node<T> {
  pub fn mutable(&self) -> NodeMutable<T> {
    let inner = modify_graph(|g| unsafe {
      let v = g.make_var(T::TYPE);
      g.write(self.handle(), v);
      v.into_node()
    });

    NodeMutable { inner }
  }
}

impl<T> NodeMutable<T> {
  pub fn set(&self, source: impl Into<Node<T>>) {
    modify_graph(|g| {
      g.write(self.inner.handle(), source.into().handle());
    })
  }
  pub fn get(&self) -> Node<T> {
    modify_graph(|g| unsafe { g.load(self.inner.handle()).into_node() })
  }
}

#[derive(Copy, Clone)]
pub struct AnyType;

impl<T> Node<T> {
  pub fn cast_untyped_node(&self) -> NodeUntyped {
    unsafe { std::mem::transmute_copy(self) }
  }
}

pub type NodeUntyped = Node<AnyType>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderGraphNodeRawHandle {
  pub handle: usize,
}

impl ShaderGraphNodeRawHandle {
  /// # Safety
  ///
  /// force type casting
  pub unsafe fn into_node<X>(&self) -> Node<X> {
    Node {
      handle: *self,
      phantom: PhantomData,
    }
  }

  pub fn into_node_untyped(&self) -> NodeUntyped {
    unsafe { self.into_node() }
  }
}
