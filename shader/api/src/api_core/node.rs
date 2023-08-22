use crate::*;

#[repr(transparent)]
pub struct Node<T: ?Sized> {
  phantom: PhantomData<T>,
  handle: ShaderNodeRawHandle,
}

impl<T: ?Sized> Clone for Node<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      handle: self.handle,
    }
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

// todo restrict type to referable?
impl<T: ShaderNodeType> Node<T> {
  pub fn make_local_var(&self) -> LocalVarNode<T> {
    call_shader_api(|g| unsafe {
      let v = g.make_local_var(T::TYPE);
      g.store(self.handle(), v);
      v.into_node()
    })
  }
}

macro_rules! impl_load {
  ($Type: tt) => {
    impl<T> $Type<T> {
      pub fn load(&self) -> Node<T> {
        call_shader_api(|g| unsafe { g.load(self.handle()).into_node() })
      }
    }
  };
}
macro_rules! impl_store {
  ($Type: tt) => {
    impl<T> $Type<T> {
      pub fn store(&self, source: impl Into<Node<T>>) {
        let source = source.into();
        call_shader_api(|g| {
          g.store(source.handle(), self.handle());
        })
      }
    }
  };
}

impl_load!(LocalVarNode);
impl_store!(LocalVarNode);

impl_load!(GlobalVarNode);
impl_store!(GlobalVarNode);

impl_load!(StorageNode);
impl_store!(StorageNode);

impl_load!(WorkGroupSharedNode);
impl_store!(WorkGroupSharedNode);

impl_load!(ReadOnlyStorageNode);
impl_load!(UniformNode);

// used in bindless
impl<T> Node<ShaderHandlePtr<ShaderHandlePtr<T>>> {
  pub fn load(&self) -> Node<ShaderHandlePtr<T>> {
    call_shader_api(|g| unsafe { g.load(self.handle()).into_node() })
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
