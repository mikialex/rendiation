use dyn_clone::DynClone;

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
pub trait AbstractShaderPtr: DynClone {
  fn field_index(&self, field_index: usize) -> Box<dyn AbstractShaderPtr>;
  fn field_array_index(&self, index: Node<u32>) -> Box<dyn AbstractShaderPtr>;
  fn array_length(&self) -> Node<u32>;
  fn load(&self) -> ShaderNodeRawHandle;
  fn store(&self, value: ShaderNodeRawHandle);
  fn downcast_atomic_ptr(&self) -> ShaderNodeRawHandle;
}

dyn_clone::clone_trait_object!(AbstractShaderPtr);

impl AbstractShaderPtr for ShaderNodeRawHandle {
  fn field_index(&self, field_index: usize) -> Box<dyn AbstractShaderPtr> {
    let node = ShaderNodeExpr::IndexStatic {
      field_index,
      target: *self,
    }
    .insert_api::<AnyType>()
    .handle();

    Box::new(node)
  }

  fn field_array_index(&self, index: Node<u32>) -> Box<dyn AbstractShaderPtr> {
    let node = OperatorNode::Index {
      array: *self,
      entry: index.handle(),
    }
    .insert_api::<AnyType>()
    .handle();

    Box::new(node)
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
  fn downcast_atomic_ptr(&self) -> ShaderNodeRawHandle {
    *self
  }
}

/// this trait is to mapping the `T` to it's typed shader access object. the access object
/// has type api to constraint valid access.
pub trait ShaderValueAbstractPtrAccess {
  type Accessor: Clone;
  fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor;
}
pub type ShaderAccessorOf<T> = <T as ShaderValueAbstractPtrAccess>::Accessor;
pub trait SizedValueShaderPtrAccessor: Clone {
  // todo, why not use abstract left trait?
  type Node: ShaderSizedValueNodeType;
  fn load(&self) -> Node<Self::Node>;
  fn store(&self, value: Node<Self::Node>);
}

pub trait SizedShaderValueAbstractPtrAccess:
  ShaderValueAbstractPtrAccess<Accessor: SizedValueShaderPtrAccessor>
{
}
impl<T> SizedShaderValueAbstractPtrAccess for T
where
  T: ShaderValueAbstractPtrAccess,
  T::Accessor: SizedValueShaderPtrAccessor,
{
}

impl<T> ShaderValueAbstractPtrAccess for [T] {
  type Accessor = DynLengthArrayAccessor<T>;
  fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor {
    DynLengthArrayAccessor {
      phantom: PhantomData,
      access: ptr,
    }
  }
}

pub struct DynLengthArrayAccessor<T> {
  phantom: PhantomData<T>,
  access: Box<dyn AbstractShaderPtr>,
}

impl<T> Clone for DynLengthArrayAccessor<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      access: self.access.clone(),
    }
  }
}

impl<T: ShaderValueAbstractPtrAccess> DynLengthArrayAccessor<T> {
  pub fn index(&self, index: Node<u32>) -> T::Accessor {
    let item = self.access.field_array_index(index);
    T::create_accessor_from_raw_ptr(item)
  }
  pub fn array_length(&self) -> Node<u32> {
    self.access.array_length()
  }
}

impl<T, const N: usize> ShaderValueAbstractPtrAccess for Shader140Array<T, N> {
  type Accessor = StaticLengthArrayAccessor<Self, T>;
  fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor {
    StaticLengthArrayAccessor {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
    }
  }
}
impl<T, const N: usize> ShaderValueAbstractPtrAccess for [T; N] {
  type Accessor = StaticLengthArrayAccessor<Self, T>;
  fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor {
    StaticLengthArrayAccessor {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
    }
  }
}

pub struct StaticLengthArrayAccessor<AT, T> {
  phantom: PhantomData<T>,
  array: PhantomData<AT>,
  access: Box<dyn AbstractShaderPtr>,
}

impl<AT, T> SizedValueShaderPtrAccessor for StaticLengthArrayAccessor<AT, T>
where
  AT: ShaderSizedValueNodeType,
  T: ShaderSizedValueNodeType,
{
  type Node = AT;
  fn load(&self) -> Node<Self::Node> {
    unsafe { self.access.load().into_node() }
  }
  fn store(&self, value: Node<Self::Node>) {
    self.access.store(value.handle())
  }
}

impl<AT, T> Clone for StaticLengthArrayAccessor<AT, T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      array: self.array,
      access: self.access.clone(),
    }
  }
}

impl<AT, T: ShaderValueAbstractPtrAccess> StaticLengthArrayAccessor<AT, T> {
  pub fn index(&self, index: Node<u32>) -> T::Accessor {
    let item = self.access.field_array_index(index);
    T::create_accessor_from_raw_ptr(item)
  }
}

pub struct DirectPrimitivePtrAccessor<T>(PhantomData<T>, Box<dyn AbstractShaderPtr>);

impl<T> Clone for DirectPrimitivePtrAccessor<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}

impl<T> SizedValueShaderPtrAccessor for DirectPrimitivePtrAccessor<T>
where
  T: ShaderSizedValueNodeType,
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
    impl ShaderValueAbstractPtrAccess for $ty {
      type Accessor = DirectPrimitivePtrAccessor<$ty>;
      fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor {
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

pub struct AtomicPtrAccessor<T>(PhantomData<T>, Box<dyn AbstractShaderPtr>);

impl<T> Clone for AtomicPtrAccessor<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}

impl<T> AtomicPtrAccessor<T> {
  /// more atomic operation is defined on raw node type, so we use this escape hatch
  /// to expose to user
  pub fn expose(&self) -> StorageNode<DeviceAtomic<T>> {
    unsafe { self.1.downcast_atomic_ptr().into_node() }
  }
}
impl<T: AtomicityShaderNodeType + PrimitiveShaderNodeType> SizedValueShaderPtrAccessor
  for AtomicPtrAccessor<T>
{
  type Node = T;

  fn load(&self) -> Node<Self::Node> {
    self.expose().atomic_load()
  }

  fn store(&self, value: Node<Self::Node>) {
    self.expose().atomic_store(value);
  }
}

impl<T> ShaderValueAbstractPtrAccess for DeviceAtomic<T>
where
  T: AtomicityShaderNodeType + PrimitiveShaderNodeType,
{
  type Accessor = AtomicPtrAccessor<T>;

  fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor {
    AtomicPtrAccessor(PhantomData, ptr)
  }
}

/// the macro expansion result demo:
#[allow(unused)]
mod test {
  use crate::*;
  pub struct MyStruct {
    pub a: f32,
    pub b: u32,
  }

  /// auto generated by macro
  pub struct MyStructShaderPtrInstance(Box<dyn AbstractShaderPtr>);
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

  impl Clone for MyStructShaderPtrInstance {
    fn clone(&self) -> Self {
      Self(self.0.clone())
    }
  }

  impl SizedValueShaderPtrAccessor for MyStructShaderPtrInstance {
    type Node = MyStruct;
    fn load(&self) -> Node<MyStruct> {
      unsafe { self.0.load().into_node() }
    }
    fn store(&self, value: Node<MyStruct>) {
      self.0.store(value.handle());
    }
  }

  /// auto generated by macro
  impl MyStructShaderPtrInstance {
    pub fn load(&self) -> Node<MyStruct> {
      unsafe { self.0.load().into_node() }
    }
    pub fn store(&self, value: Node<MyStruct>) {
      self.0.store(value.handle());
    }

    pub fn a(&self) -> ShaderAccessorOf<f32> {
      let v = self.0.field_index(0);
      f32::create_accessor_from_raw_ptr(v)
    }
    pub fn b(&self) -> ShaderAccessorOf<u32> {
      let v = self.0.field_index(1);
      u32::create_accessor_from_raw_ptr(v)
    }
  }

  impl ShaderValueAbstractPtrAccess for MyStruct {
    type Accessor = MyStructShaderPtrInstance;
    fn create_accessor_from_raw_ptr(ptr: Box<dyn AbstractShaderPtr>) -> Self::Accessor {
      MyStructShaderPtrInstance(ptr)
    }
  }
}
