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
/// do anything(like direct panic) or do nothing in silence because validation error will eventually
///  raised in later process.
///
/// todo, mark unsafe fn
pub trait AbstractShaderPtr: DynClone {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr;
  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr;
  fn array_length(&self) -> Node<u32>;
  fn load(&self) -> ShaderNodeRawHandle;
  fn store(&self, value: ShaderNodeRawHandle);
  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle;
}
pub type BoxedShaderPtr = Box<dyn AbstractShaderPtr>;

dyn_clone::clone_trait_object!(AbstractShaderPtr);

impl AbstractShaderPtr for ShaderNodeRawHandle {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    let node = ShaderNodeExpr::IndexStatic {
      field_index,
      target: *self,
    }
    .insert_api_raw();

    Box::new(node)
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let node = OperatorNode::Index {
      array: *self,
      entry: index.handle(),
    }
    .insert_api_raw();

    Box::new(node)
  }

  fn array_length(&self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::ArrayLength, [*self])
  }

  fn load(&self) -> ShaderNodeRawHandle {
    call_shader_api(|g| g.load(*self))
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    call_shader_api(|g| g.store(value, *self))
  }
  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    *self
  }
}

/// this trait is to mapping the `T` to it's typed shader access object. the access object
/// has type api to constraint valid access.
pub trait ShaderAbstractPtrAccess {
  type PtrView: Clone;
  type ReadonlyPtrView: Clone;
  // todo, this fn should be unsafe
  fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView;
  fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView;
}
pub type ShaderPtrOf<T> = <T as ShaderAbstractPtrAccess>::PtrView;
pub type ShaderReadonlyPtrOf<T> = <T as ShaderAbstractPtrAccess>::ReadonlyPtrView;

/// the difference of the trait between the abstract left value is that the abstract
/// left value's right value is not necessarily one single shader value.
pub trait ReadonlySizedShaderPtrView: Clone {
  type Node: ShaderSizedValueNodeType;
  fn load(&self) -> Node<Self::Node>;
}
pub trait SizedShaderPtrView: ReadonlySizedShaderPtrView {
  fn store(&self, value: impl Into<Node<Self::Node>>);
}

pub trait SizedShaderAbstractPtrAccess:
  ShaderAbstractPtrAccess<
  PtrView: SizedShaderPtrView<Node = Self>,
  ReadonlyPtrView: ReadonlySizedShaderPtrView<Node = Self>,
>
{
}
impl<T> SizedShaderAbstractPtrAccess for T
where
  T: ShaderAbstractPtrAccess,
  T::PtrView: SizedShaderPtrView<Node = Self>,
  T::ReadonlyPtrView: ReadonlySizedShaderPtrView<Node = Self>,
{
}

impl<T> ShaderAbstractPtrAccess for [T] {
  type PtrView = DynLengthArrayView<T>;
  type ReadonlyPtrView = DynLengthArrayReadonlyView<T>;
  fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
    DynLengthArrayView {
      phantom: PhantomData,
      access: ptr,
    }
  }

  fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
    DynLengthArrayReadonlyView {
      phantom: PhantomData,
      access: ptr,
    }
  }
}

/// enable this when mysterious device lost happens randomly.
/// check if the crash is due to the out of bound access by crashing the device deterministically
const ENABLE_STORAGE_BUFFER_BOUND_CHECK: bool = true;

pub struct DynLengthArrayView<T> {
  phantom: PhantomData<T>,
  access: BoxedShaderPtr,
}
impl<T> Clone for DynLengthArrayView<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      access: self.access.clone(),
    }
  }
}
impl<T: SizedShaderAbstractPtrAccess> DynLengthArrayView<T> {
  pub fn index(&self, index: impl Into<Node<u32>>) -> T::PtrView {
    let index = index.into();
    if ENABLE_STORAGE_BUFFER_BOUND_CHECK {
      shader_assert(index.less_than(self.array_length()));
    }
    let item = self.access.field_array_index(index);
    T::create_view_from_raw_ptr(item)
  }
  pub fn array_length(&self) -> Node<u32> {
    self.access.array_length()
  }
}

pub struct DynLengthArrayReadonlyView<T> {
  phantom: PhantomData<T>,
  access: BoxedShaderPtr,
}
impl<T> Clone for DynLengthArrayReadonlyView<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      access: self.access.clone(),
    }
  }
}
impl<T: SizedShaderAbstractPtrAccess> DynLengthArrayReadonlyView<T> {
  pub fn index(&self, index: impl Into<Node<u32>>) -> T::ReadonlyPtrView {
    let index = index.into();
    if ENABLE_STORAGE_BUFFER_BOUND_CHECK {
      shader_assert(index.less_than(self.array_length()));
    }
    let item = self.access.field_array_index(index);
    T::create_readonly_view_from_raw_ptr(item)
  }
  pub fn array_length(&self) -> Node<u32> {
    self.access.array_length()
  }
}

impl<T, const N: usize> ShaderAbstractPtrAccess for Shader140Array<T, N> {
  type PtrView = StaticLengthArrayView<Self, T>;
  type ReadonlyPtrView = StaticLengthArrayReadonlyView<Self, T>;
  fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
    StaticLengthArrayView {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
      len: N as u32,
    }
  }
  fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
    StaticLengthArrayReadonlyView {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
      len: N as u32,
    }
  }
}
impl<T, const N: usize> ShaderAbstractPtrAccess for [T; N] {
  type PtrView = StaticLengthArrayView<Self, T>;
  type ReadonlyPtrView = StaticLengthArrayReadonlyView<Self, T>;
  fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
    StaticLengthArrayView {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
      len: N as u32,
    }
  }
  fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
    StaticLengthArrayReadonlyView {
      phantom: PhantomData,
      array: PhantomData,
      access: ptr,
      len: N as u32,
    }
  }
}
impl<T> ShaderAbstractPtrAccess for HostDynSizeArray<T> {
  type PtrView = StaticLengthArrayView<Self, T>;
  type ReadonlyPtrView = StaticLengthArrayReadonlyView<Self, T>;
  fn create_view_from_raw_ptr(_: BoxedShaderPtr) -> Self::PtrView {
    panic!("this fn should called from ptr builder side as we don't know the length")
  }
  fn create_readonly_view_from_raw_ptr(_: BoxedShaderPtr) -> Self::ReadonlyPtrView {
    panic!("this fn should called from ptr builder side as we don't know the length")
  }
}

pub struct StaticLengthArrayView<AT, T> {
  pub phantom: PhantomData<T>,
  pub array: PhantomData<AT>,
  pub access: BoxedShaderPtr,
  pub len: u32,
}

impl<AT, T> ReadonlySizedShaderPtrView for StaticLengthArrayView<AT, T>
where
  AT: ShaderSizedValueNodeType,
  T: ShaderSizedValueNodeType,
{
  type Node = AT;
  fn load(&self) -> Node<Self::Node> {
    unsafe { self.access.load().into_node() }
  }
}
impl<AT, T> SizedShaderPtrView for StaticLengthArrayView<AT, T>
where
  AT: ShaderSizedValueNodeType,
  T: ShaderSizedValueNodeType,
{
  fn store(&self, value: impl Into<Node<Self::Node>>) {
    self.access.store(value.into().handle())
  }
}
impl<AT, T> Clone for StaticLengthArrayView<AT, T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      array: self.array,
      access: self.access.clone(),
      len: self.len,
    }
  }
}
impl<AT, T: SizedShaderAbstractPtrAccess> StaticLengthArrayView<AT, T> {
  pub fn index(&self, index: impl Into<Node<u32>>) -> T::PtrView {
    let item = self.access.field_array_index(index.into());
    T::create_view_from_raw_ptr(item)
  }
}

pub struct StaticLengthArrayReadonlyView<AT, T> {
  phantom: PhantomData<T>,
  array: PhantomData<AT>,
  access: BoxedShaderPtr,
  pub len: u32,
}

impl<AT, T> ReadonlySizedShaderPtrView for StaticLengthArrayReadonlyView<AT, T>
where
  AT: ShaderSizedValueNodeType,
  T: ShaderSizedValueNodeType,
{
  type Node = AT;
  fn load(&self) -> Node<Self::Node> {
    unsafe { self.access.load().into_node() }
  }
}
impl<AT, T> Clone for StaticLengthArrayReadonlyView<AT, T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      array: self.array,
      access: self.access.clone(),
      len: self.len,
    }
  }
}
impl<AT, T: SizedShaderAbstractPtrAccess> StaticLengthArrayReadonlyView<AT, T> {
  pub fn index(&self, index: impl Into<Node<u32>>) -> T::ReadonlyPtrView {
    let item = self.access.field_array_index(index.into());
    T::create_readonly_view_from_raw_ptr(item)
  }
}

pub struct DirectPrimitivePtrView<T>(PhantomData<T>, BoxedShaderPtr);

impl<T> Clone for DirectPrimitivePtrView<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}
impl<T> ReadonlySizedShaderPtrView for DirectPrimitivePtrView<T>
where
  T: ShaderSizedValueNodeType,
{
  type Node = T;
  fn load(&self) -> Node<T> {
    unsafe { self.1.load().into_node() }
  }
}
impl<T> SizedShaderPtrView for DirectPrimitivePtrView<T>
where
  T: ShaderSizedValueNodeType,
{
  fn store(&self, value: impl Into<Node<T>>) {
    self.1.store(value.into().handle());
  }
}
pub struct ReadonlyDirectPrimitivePtrView<T>(PhantomData<T>, BoxedShaderPtr);

impl<T> Clone for ReadonlyDirectPrimitivePtrView<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}
impl<T> ReadonlySizedShaderPtrView for ReadonlyDirectPrimitivePtrView<T>
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
    impl ShaderAbstractPtrAccess for $ty {
      type PtrView = DirectPrimitivePtrView<$ty>;
      type ReadonlyPtrView = ReadonlyDirectPrimitivePtrView<$ty>;
      fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
        DirectPrimitivePtrView(PhantomData, ptr)
      }
      fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
        ReadonlyDirectPrimitivePtrView(PhantomData, ptr)
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

pub struct AtomicPtrView<T>(PhantomData<T>, BoxedShaderPtr);

impl<T> Clone for AtomicPtrView<T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1.clone())
  }
}

impl<T: AtomicityShaderNodeType> AtomicPtrView<T> {
  pub fn atomic_load(&self) -> Node<T> {
    call_shader_api(|g| unsafe { g.load(self.1.get_self_atomic_ptr()).into_node() })
  }
  pub fn atomic_store(&self, v: Node<T>) {
    call_shader_api(|g| g.store(v.handle(), self.1.get_self_atomic_ptr()))
  }

  pub fn atomic_add(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::Add,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_sub(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::Subtract,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_min(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::Min,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_max(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::Max,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_and(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::And,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_or(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::InclusiveOr,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_xor(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::ExclusiveOr,
      value: v.handle(),
    }
    .insert_api()
  }
  pub fn atomic_exchange(&self, v: Node<T>) -> Node<T> {
    ShaderNodeExpr::AtomicCall {
      ty: T::ATOM,
      pointer: self.1.get_self_atomic_ptr(),
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
      pointer: self.1.get_self_atomic_ptr(),
      function: AtomicFunction::Exchange {
        compare: None,
        weak: false,
      },
      value: v.handle(),
    }
    .insert_api_raw();

    unsafe {
      let old = index_access_field(raw, 0).into_node();
      let exchanged = index_access_field(raw, 1).into_node();
      (old, exchanged)
    }
  }
}
impl<T: AtomicityShaderNodeType + PrimitiveShaderNodeType> ReadonlySizedShaderPtrView
  for AtomicPtrView<T>
{
  type Node = DeviceAtomic<T>;

  fn load(&self) -> Node<Self::Node> {
    unreachable!("atomic is not able to direct load");
  }
}
impl<T: AtomicityShaderNodeType + PrimitiveShaderNodeType> SizedShaderPtrView for AtomicPtrView<T> {
  fn store(&self, _value: impl Into<Node<Self::Node>>) {
    unreachable!("atomic is not able to direct store");
  }
}

impl<T> ShaderAbstractPtrAccess for DeviceAtomic<T>
where
  T: AtomicityShaderNodeType + PrimitiveShaderNodeType,
{
  type PtrView = AtomicPtrView<T>;
  type ReadonlyPtrView = AtomicPtrView<T>;

  fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
    AtomicPtrView(PhantomData, ptr)
  }

  fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
    AtomicPtrView(PhantomData, ptr)
  }
}
