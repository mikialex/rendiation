use crate::*;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Node<T> {
  pub(crate) phantom: PhantomData<T>,
  pub(crate) handle: ShaderNodeRawHandle,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderNodeRawHandle {
    self.handle
  }

  /// # Safety
  /// force type casting
  pub unsafe fn cast_type<X>(self) -> Node<X>
  where
    X: ShaderNodeType,
  {
    std::mem::transmute(self)
  }
}

impl<T> From<T> for Node<T>
where
  T: PrimitiveShaderNodeType,
{
  fn from(input: T) -> Self {
    ShaderNodeExpr::Const(ConstNode {
      data: input.to_primitive(),
    })
    .insert_api()
  }
}

pub struct NodeMutable<T> {
  pub(crate) inner: Node<T>,
}

impl<T: ShaderNodeType> Node<T> {
  pub fn mutable(&self) -> NodeMutable<T> {
    let inner = call_shader_api(|g| unsafe {
      let v = g.make_var(T::TYPE);
      g.store(self.handle(), v);
      v.into_node()
    });

    NodeMutable { inner }
  }
}

impl<T> NodeMutable<T> {
  pub fn set(&self, source: impl Into<Node<T>>) {
    call_shader_api(|g| {
      g.store(self.inner.handle(), source.into().handle());
    })
  }
  pub fn get(&self) -> Node<T> {
    call_shader_api(|g| unsafe { g.load(self.inner.handle()).into_node() })
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
pub struct ShaderNodeRawHandle {
  pub handle: usize,
}

impl ShaderNodeRawHandle {
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
