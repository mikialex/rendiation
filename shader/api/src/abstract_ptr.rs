use crate::*;

/// this trait is to abstract the interface of different shader ptr types, for example
/// StorageNode, UniformNode, or **user defined ptr-like object**
///
/// the implementation should be able to implement Clone because every shader node can be cloned
/// so any shader runtime-like object should be clonable as well.
///
/// as this trait is untyped, the method may not valid to implement for some type, for example
/// it's impossible to array index to a none-array object. when this case happens, the method could
/// do anything in silence because validation error will eventually raised in later process.
///
/// todo, separate store ability
pub trait AbstractShaderPtr: Clone {
  fn field_index(&self, field_index: usize) -> Self;
  fn field_array_index(&self, index: Node<u32>) -> Self;
  fn array_length(&self) -> Node<u32>;
  fn load(&self) -> ShaderNodeRawHandle;
  fn store(&self, value: ShaderNodeRawHandle);
  fn self_node(&self) -> ShaderNodeRawHandle;
}

impl AbstractShaderPtr for ShaderNodeRawHandle {
  fn field_index(&self, field_index: usize) -> Self {
    ShaderNodeExpr::IndexStatic {
      field_index,
      target: *self,
    }
    .insert_api::<AnyType>()
    .handle()
  }

  fn field_array_index(&self, index: Node<u32>) -> Self {
    OperatorNode::Index {
      array: *self,
      entry: index.handle(),
    }
    .insert_api::<AnyType>()
    .handle()
  }

  fn array_length(&self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::ArrayLength, [*self])
  }

  fn load(&self) -> ShaderNodeRawHandle {
    call_shader_api(|g| g.load(*self))
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    call_shader_api(|g| g.store(*self, value))
  }
  fn self_node(&self) -> ShaderNodeRawHandle {
    *self
  }
}

/// this trait is to mapping the `T` to it's typed shader access object. the access object
/// has type api to constraint valid access.
pub trait ShaderValueAbstractPtrAccess<Ptr: AbstractShaderPtr> {
  type Accessor: Clone;
  fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor;
}
pub type ShaderAccessorOf<T, Ptr> = <T as ShaderValueAbstractPtrAccess<Ptr>>::Accessor;
pub trait SizedValueShaderPtrAccessor: Clone {
  type Node: ShaderSizedValueNodeType;
  fn load(&self) -> Node<Self::Node>;
  fn store(&self, value: Node<Self::Node>);
}

pub trait SizedShaderValueAbstractPtrAccess<Ptr: AbstractShaderPtr>:
  ShaderValueAbstractPtrAccess<Ptr, Accessor: SizedValueShaderPtrAccessor<Node = Self>>
{
}
impl<T, Ptr: AbstractShaderPtr> SizedShaderValueAbstractPtrAccess<Ptr> for T
where
  T: ShaderValueAbstractPtrAccess<Ptr>,
  T::Accessor: SizedValueShaderPtrAccessor<Node = T>,
{
}

impl<T, Ptr: AbstractShaderPtr> ShaderValueAbstractPtrAccess<Ptr> for [T] {
  type Accessor = PointerArrayAccessor<T, Ptr>;
  fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor {
    PointerArrayAccessor {
      phantom: PhantomData,
      access: ptr,
    }
  }
}

// todo, fix array length not exist
impl<T, Ptr: AbstractShaderPtr, const N: usize> ShaderValueAbstractPtrAccess<Ptr>
  for Shader140Array<T, N>
{
  type Accessor = PointerArrayAccessor<T, Ptr>;
  fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor {
    PointerArrayAccessor {
      phantom: PhantomData,
      access: ptr,
    }
  }
}

pub struct PointerArrayAccessor<T, Ptr> {
  phantom: PhantomData<T>,
  access: Ptr,
}

impl<T, Ptr: Clone> Clone for PointerArrayAccessor<T, Ptr> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      access: self.access.clone(),
    }
  }
}

impl<T: ShaderValueAbstractPtrAccess<Ptr>, Ptr: AbstractShaderPtr> PointerArrayAccessor<T, Ptr> {
  pub fn index(&self, index: Node<u32>) -> T::Accessor {
    let item = self.access.field_array_index(index);
    T::create_accessor_from_raw_ptr(item)
  }
  pub fn array_length(&self) -> Node<u32> {
    self.access.array_length()
  }
}

pub struct DirectPrimitivePtrAccessor<T, Ptr>(PhantomData<T>, Ptr);

impl<T, Ptr: Clone> Clone for DirectPrimitivePtrAccessor<T, Ptr> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}

impl<T, Ptr> SizedValueShaderPtrAccessor for DirectPrimitivePtrAccessor<T, Ptr>
where
  T: ShaderSizedValueNodeType,
  Ptr: AbstractShaderPtr,
{
  type Node = T;
  fn load(&self) -> Node<T> {
    unsafe { self.1.load().into_node() }
  }
  fn store(&self, value: Node<T>) {
    self.1.store(value.handle());
  }
}

macro_rules! impl_primitive_with_vec_direct {
  ($ty: ty) => {
    impl_primitive_direct!($ty);
    impl_primitive_direct!(Vec2<$ty>);
    impl_primitive_direct!(Vec3<$ty>);
    impl_primitive_direct!(Vec4<$ty>);
  };
}

macro_rules! impl_primitive_mat_direct {
  ($ty: ty) => {
    impl_primitive_direct!(Mat2<$ty>);
    impl_primitive_direct!(Mat3<$ty>);
    impl_primitive_direct!(Mat4<$ty>);
  };
}

macro_rules! impl_primitive_direct {
  ($ty: ty) => {
    impl<Ptr: AbstractShaderPtr> ShaderValueAbstractPtrAccess<Ptr> for $ty {
      type Accessor = DirectPrimitivePtrAccessor<$ty, Ptr>;
      fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor {
        DirectPrimitivePtrAccessor(PhantomData, ptr)
      }
    }
  };
}

impl_primitive_direct!(Bool);
impl_primitive_with_vec_direct!(bool);
impl_primitive_with_vec_direct!(i32);
impl_primitive_with_vec_direct!(u32);
impl_primitive_with_vec_direct!(f32);
impl_primitive_mat_direct!(f32);

pub struct AtomicPtrAccessor<T, Ptr>(PhantomData<T>, Ptr);

impl<T, Ptr: Clone> Clone for AtomicPtrAccessor<T, Ptr> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}

impl<T, Ptr: AbstractShaderPtr> AtomicPtrAccessor<T, Ptr> {
  pub fn expose(&self) -> StorageNode<DeviceAtomic<T>> {
    unsafe { self.1.self_node().into_node() }
  }
}

macro_rules! impl_atomic_primitive_direct {
  ($ty: ty) => {
    impl<Ptr: AbstractShaderPtr> ShaderValueAbstractPtrAccess<Ptr> for DeviceAtomic<$ty> {
      type Accessor = AtomicPtrAccessor<$ty, Ptr>;
      fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor {
        AtomicPtrAccessor(PhantomData, ptr)
      }
    }
  };
}
impl_atomic_primitive_direct!(u32);
impl_atomic_primitive_direct!(i32);

/// the macro expansion result demo:
#[allow(unused)]
mod test {
  use crate::*;
  pub struct MyStruct {
    pub a: f32,
    pub b: u32,
  }

  /// auto generated by macro
  pub struct MyStructShaderPtrInstance<Ptr>(Ptr);
  impl ShaderNodeType for MyStruct {
    fn ty() -> ShaderValueType {
      todo!()
    }
  }
  impl ShaderSizedValueNodeType for MyStruct {
    fn sized_ty() -> ShaderSizedValueType {
      todo!()
    }

    fn to_value(&self) -> ShaderStructFieldInitValue {
      todo!()
    }
  }

  impl<Ptr: Clone> Clone for MyStructShaderPtrInstance<Ptr> {
    fn clone(&self) -> Self {
      Self(self.0.clone())
    }
  }

  impl<Ptr: AbstractShaderPtr> SizedValueShaderPtrAccessor for MyStructShaderPtrInstance<Ptr> {
    type Node = MyStruct;
    fn load(&self) -> Node<MyStruct> {
      unsafe { self.0.load().into_node() }
    }
    fn store(&self, value: Node<MyStruct>) {
      self.0.store(value.handle());
    }
  }

  /// auto generated by macro
  impl<Ptr: AbstractShaderPtr> MyStructShaderPtrInstance<Ptr> {
    pub fn load(&self) -> Node<MyStruct> {
      unsafe { self.0.load().into_node() }
    }
    pub fn store(&self, value: Node<MyStruct>) {
      self.0.store(value.handle());
    }

    pub fn a(&self) -> ShaderAccessorOf<f32, Ptr> {
      let v = self.0.field_index(0);
      f32::create_accessor_from_raw_ptr(v)
    }
    pub fn b(&self) -> ShaderAccessorOf<u32, Ptr> {
      let v = self.0.field_index(1);
      u32::create_accessor_from_raw_ptr(v)
    }
  }

  impl<Ptr: AbstractShaderPtr> ShaderValueAbstractPtrAccess<Ptr> for MyStruct {
    type Accessor = MyStructShaderPtrInstance<Ptr>;
    fn create_accessor_from_raw_ptr(ptr: Ptr) -> Self::Accessor {
      MyStructShaderPtrInstance(ptr)
    }
  }
}
