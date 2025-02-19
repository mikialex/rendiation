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
///
/// todo, mark unsafe fn
pub trait AbstractShaderPtr: DynClone {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr;
  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr;
  fn array_length(&self) -> Node<u32>;
  fn load(&self) -> ShaderNodeRawHandle;
  fn store(&self, value: ShaderNodeRawHandle);
  fn downcast_atomic_ptr(&self) -> ShaderNodeRawHandle;
}
pub type BoxedShaderPtr = Box<dyn AbstractShaderPtr>;

dyn_clone::clone_trait_object!(AbstractShaderPtr);

impl AbstractShaderPtr for ShaderNodeRawHandle {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    let node = ShaderNodeExpr::IndexStatic {
      field_index,
      target: *self,
    }
    .insert_api::<AnyType>()
    .handle();

    Box::new(node)
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
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
  type ReadonlyAccessor: Clone;
  // todo, this fn should be unsafe
  fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor;
  fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor;
}
pub type ShaderAccessorOf<T> = <T as ShaderValueAbstractPtrAccess>::Accessor;
pub type ShaderReadonlyAccessorOf<T> = <T as ShaderValueAbstractPtrAccess>::ReadonlyAccessor;

/// the difference of the trait between the abstract left value is that the abstract
/// left value's right value is not necessarily one single shader value.
pub trait ReadonlySizedValueShaderPtrAccessor: Clone {
  type Node: ShaderSizedValueNodeType;
  fn load(&self) -> Node<Self::Node>;
}
pub trait SizedValueShaderPtrAccessor: ReadonlySizedValueShaderPtrAccessor {
  fn store(&self, value: impl Into<Node<Self::Node>>);
}

pub trait SizedShaderValueAbstractPtrAccess:
  ShaderValueAbstractPtrAccess<
  Accessor: SizedValueShaderPtrAccessor<Node = Self>,
  ReadonlyAccessor: ReadonlySizedValueShaderPtrAccessor<Node = Self>,
>
{
}
impl<T> SizedShaderValueAbstractPtrAccess for T
where
  T: ShaderValueAbstractPtrAccess,
  T::Accessor: SizedValueShaderPtrAccessor<Node = Self>,
  T::ReadonlyAccessor: ReadonlySizedValueShaderPtrAccessor<Node = Self>,
{
}

impl<T> ShaderValueAbstractPtrAccess for [T] {
  type Accessor = DynLengthArrayAccessor<T>;
  type ReadonlyAccessor = DynLengthArrayAccessor<T>;
  fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
    DynLengthArrayAccessor {
      phantom: PhantomData,
      access: ptr,
    }
  }

  fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
    Self::create_accessor_from_raw_ptr(ptr)
  }
}

pub struct DynLengthArrayAccessor<T> {
  phantom: PhantomData<T>,
  access: BoxedShaderPtr,
}

impl<T> Clone for DynLengthArrayAccessor<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      access: self.access.clone(),
    }
  }
}

impl<T: SizedShaderValueAbstractPtrAccess> DynLengthArrayAccessor<T> {
  pub fn index(&self, index: impl Into<Node<u32>>) -> T::Accessor {
    let item = self.access.field_array_index(index.into());
    T::create_accessor_from_raw_ptr(item)
  }
  pub fn array_length(&self) -> Node<u32> {
    self.access.array_length()
  }
}

impl<T, const N: usize> ShaderValueAbstractPtrAccess for Shader140Array<T, N> {
  type Accessor = StaticLengthArrayAccessor<Self, T>;
  type ReadonlyAccessor = StaticLengthArrayAccessor<Self, T>;
  fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
    StaticLengthArrayAccessor {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
    }
  }
  fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor {
    Self::create_accessor_from_raw_ptr(ptr)
  }
}
impl<T, const N: usize> ShaderValueAbstractPtrAccess for [T; N] {
  type Accessor = StaticLengthArrayAccessor<Self, T>;
  type ReadonlyAccessor = StaticLengthArrayAccessor<Self, T>;
  fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
    StaticLengthArrayAccessor {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
    }
  }
  fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor {
    Self::create_accessor_from_raw_ptr(ptr)
  }
}
impl<T> ShaderValueAbstractPtrAccess for HostDynSizeArray<T> {
  type Accessor = StaticLengthArrayAccessor<Self, T>;
  type ReadonlyAccessor = StaticLengthArrayAccessor<Self, T>;
  fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
    StaticLengthArrayAccessor {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
    }
  }
  fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor {
    Self::create_accessor_from_raw_ptr(ptr)
  }
}

pub struct StaticLengthArrayAccessor<AT, T> {
  phantom: PhantomData<T>,
  array: PhantomData<AT>,
  access: BoxedShaderPtr,
}

impl<AT, T> ReadonlySizedValueShaderPtrAccessor for StaticLengthArrayAccessor<AT, T>
where
  AT: ShaderSizedValueNodeType,
  T: ShaderSizedValueNodeType,
{
  type Node = AT;
  fn load(&self) -> Node<Self::Node> {
    unsafe { self.access.load().into_node() }
  }
}
impl<AT, T> SizedValueShaderPtrAccessor for StaticLengthArrayAccessor<AT, T>
where
  AT: ShaderSizedValueNodeType,
  T: ShaderSizedValueNodeType,
{
  fn store(&self, value: impl Into<Node<Self::Node>>) {
    self.access.store(value.into().handle())
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

impl<AT, T: SizedShaderValueAbstractPtrAccess> StaticLengthArrayAccessor<AT, T> {
  pub fn index(&self, index: impl Into<Node<u32>>) -> T::Accessor {
    let item = self.access.field_array_index(index.into());
    T::create_accessor_from_raw_ptr(item)
  }
}

pub struct DirectPrimitivePtrAccessor<T>(PhantomData<T>, BoxedShaderPtr);

impl<T> Clone for DirectPrimitivePtrAccessor<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}
impl<T> ReadonlySizedValueShaderPtrAccessor for DirectPrimitivePtrAccessor<T>
where
  T: ShaderSizedValueNodeType,
{
  type Node = T;
  fn load(&self) -> Node<T> {
    unsafe { self.1.load().into_node() }
  }
}
impl<T> SizedValueShaderPtrAccessor for DirectPrimitivePtrAccessor<T>
where
  T: ShaderSizedValueNodeType,
{
  fn store(&self, value: impl Into<Node<T>>) {
    self.1.store(value.into().handle());
  }
}
pub struct ReadonlyDirectPrimitivePtrAccessor<T>(PhantomData<T>, BoxedShaderPtr);

impl<T> Clone for ReadonlyDirectPrimitivePtrAccessor<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}
impl<T> ReadonlySizedValueShaderPtrAccessor for ReadonlyDirectPrimitivePtrAccessor<T>
where
  T: ShaderSizedValueNodeType,
{
  type Node = T;
  fn load(&self) -> Node<T> {
    unsafe { self.1.load().into_node() }
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
      type ReadonlyAccessor = ReadonlyDirectPrimitivePtrAccessor<$ty>;
      fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
        DirectPrimitivePtrAccessor(PhantomData, ptr)
      }
      fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor {
        ReadonlyDirectPrimitivePtrAccessor(PhantomData, ptr)
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

pub struct AtomicPtrAccessor<T>(PhantomData<T>, BoxedShaderPtr);

impl<T> Clone for AtomicPtrAccessor<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}

impl<T: AtomicityShaderNodeType> AtomicPtrAccessor<T> {
  pub fn atomic_load(&self) -> Node<T> {
    call_shader_api(|g| unsafe { g.load(self.1.downcast_atomic_ptr()).into_node() })
  }
  pub fn atomic_store(&self, v: Node<T>) {
    call_shader_api(|g| g.store(v.handle(), self.1.downcast_atomic_ptr()))
  }

  pub fn atomic_add(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::Add,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_sub(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::Subtract,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_min(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::Min,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_max(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::Max,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_and(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::And,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_or(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::InclusiveOr,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_xor(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::ExclusiveOr,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_exchange(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::Exchange {
        compare: None,
        weak: false,
      },
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_exchange_weak(&self, v: Node<T>) -> (Node<T>, Node<bool>) {
    let raw = ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.downcast_atomic_ptr(),
      function: AtomicFunction::Exchange {
        compare: None,
        weak: false,
      },
      value: v.handle(),
    }
    .insert_api::<AnyType>()
    .handle();

    unsafe {
      let old = index_access_field(raw, 0).into_node();
      let exchanged = index_access_field(raw, 1).into_node();
      (old, exchanged)
    }
  }
}
impl<T: AtomicityShaderNodeType + PrimitiveShaderNodeType> ReadonlySizedValueShaderPtrAccessor
  for AtomicPtrAccessor<T>
{
  type Node = DeviceAtomic<T>;

  fn load(&self) -> Node<Self::Node> {
    unreachable!("atomic is not able to direct load");
  }
}
impl<T: AtomicityShaderNodeType + PrimitiveShaderNodeType> SizedValueShaderPtrAccessor
  for AtomicPtrAccessor<T>
{
  fn store(&self, _value: impl Into<Node<Self::Node>>) {
    unreachable!("atomic is not able to direct store");
  }
}

impl<T> ShaderValueAbstractPtrAccess for DeviceAtomic<T>
where
  T: AtomicityShaderNodeType + PrimitiveShaderNodeType,
{
  type Accessor = AtomicPtrAccessor<T>;
  type ReadonlyAccessor = AtomicPtrAccessor<T>;

  fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
    AtomicPtrAccessor(PhantomData, ptr)
  }

  fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor {
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

  pub struct MyStructShaderPtrInstance(BoxedShaderPtr);

  impl Clone for MyStructShaderPtrInstance {
    fn clone(&self) -> Self {
      Self(self.0.clone())
    }
  }

  impl ReadonlySizedValueShaderPtrAccessor for MyStructShaderPtrInstance {
    type Node = MyStruct;
    fn load(&self) -> Node<MyStruct> {
      unsafe { self.0.load().into_node() }
    }
  }
  impl SizedValueShaderPtrAccessor for MyStructShaderPtrInstance {
    fn store(&self, value: impl Into<Node<MyStruct>>) {
      self.0.store(value.into().handle());
    }
  }

  /// auto generated by macro
  impl MyStructShaderPtrInstance {
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

  pub struct MyStructShaderReadonlyPtrInstance(BoxedShaderPtr);

  impl Clone for MyStructShaderReadonlyPtrInstance {
    fn clone(&self) -> Self {
      Self(self.0.clone())
    }
  }

  impl ReadonlySizedValueShaderPtrAccessor for MyStructShaderReadonlyPtrInstance {
    type Node = MyStruct;
    fn load(&self) -> Node<MyStruct> {
      unsafe { self.0.load().into_node() }
    }
  }

  /// auto generated by macro
  impl MyStructShaderReadonlyPtrInstance {
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
    type ReadonlyAccessor = MyStructShaderReadonlyPtrInstance;
    fn create_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::Accessor {
      MyStructShaderPtrInstance(ptr)
    }
    fn create_readonly_accessor_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyAccessor {
      MyStructShaderReadonlyPtrInstance(ptr)
    }
  }
}
