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

macro_rules! atomic_impls {
  ($NodeType: tt) => {
    impl<T: AtomicityShaderNodeType> $NodeType<DeviceAtomic<T>> {
      pub fn atomic_load(&self) -> Node<T> {
        call_shader_api(|g| unsafe { g.load(self.handle()).into_node() })
      }
      pub fn atomic_store(&self, v: Node<T>) {
        call_shader_api(|g| g.store(v.handle(), self.handle()))
      }

      pub fn atomic_add(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::Add,
          value: v.handle(),
        }
        .insert_api()
      }
      pub fn atomic_sub(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::Subtract,
          value: v.handle(),
        }
        .insert_api()
      }
      pub fn atomic_min(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::Min,
          value: v.handle(),
        }
        .insert_api()
      }
      pub fn atomic_max(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::Max,
          value: v.handle(),
        }
        .insert_api()
      }
      pub fn atomic_and(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::And,
          value: v.handle(),
        }
        .insert_api()
      }
      pub fn atomic_or(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::InclusiveOr,
          value: v.handle(),
        }
        .insert_api()
      }
      pub fn atomic_xor(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::ExclusiveOr,
          value: v.handle(),
        }
        .insert_api()
      }
      // todo, compare exchange weak
      pub fn atomic_exchange(&self, v: Node<T>) -> Node<T> {
        ShaderNodeExpr::AtomicCall {
          ty: T::ATOM,
          pointer: self.handle(),
          function: AtomicFunction::Exchange { compare: None },
          value: v.handle(),
        }
        .insert_api()
      }
    }
  };
}

atomic_impls!(WorkGroupSharedNode);
atomic_impls!(StorageNode);

// todo restrict type
pub fn make_local_var<T: ShaderNodeType>() -> LocalVarNode<T> {
  call_shader_api(|g| unsafe {
    let v = g.make_local_var(T::TYPE);
    v.into_node()
  })
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
