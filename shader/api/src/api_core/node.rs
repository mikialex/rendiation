use crate::*;

#[repr(transparent)]
pub struct Node<T: ?Sized> {
  phantom: PhantomData<T>,
  handle: ShaderNodeRawHandle,
}

impl<T: ?Sized> Clone for Node<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T: ?Sized> Copy for Node<T> {}

impl<T: ?Sized> Node<T> {
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
    ShaderNodeExpr::Const {
      data: input.to_primitive(),
    }
    .insert_api()
  }
}

pub fn make_local_var<T: ShaderSizedValueNodeType>() -> ShaderPtrOf<T> {
  let handle = call_shader_api(|g| g.make_local_var(T::ty()));
  T::create_view_from_raw_ptr(Box::new(handle))
}

impl<T: ShaderSizedValueNodeType> Node<T> {
  pub fn make_local_var(&self) -> ShaderPtrOf<T> {
    let handle = call_shader_api(|g| {
      let v = g.make_local_var(T::ty());
      g.store(self.handle(), v);
      v
    });
    T::create_view_from_raw_ptr(Box::new(handle))
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

impl<T: ShaderSizedValueNodeType> Default for Node<T> {
  fn default() -> Self {
    zeroed_val()
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderNodeRawHandle {
  pub handle: usize,
}

impl ShaderNodeRawHandle {
  /// # Safety
  ///
  /// force type casting
  pub unsafe fn into_node<X: ?Sized>(&self) -> Node<X> {
    Node {
      handle: *self,
      phantom: PhantomData,
    }
  }

  pub fn into_node_untyped(&self) -> NodeUntyped {
    unsafe { self.into_node() }
  }
}
