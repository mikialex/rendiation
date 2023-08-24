use crate::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderStages {
  Vertex,
  Fragment,
  Compute,
}

/// https://www.w3.org/TR/WGSL/#address-space
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddressSpace {
  /// Function locals.
  Function,
  /// Private data, per invocation, mutable.
  Private,
  /// Workgroup shared data, mutable.
  WorkGroup,
  /// Uniform buffer data.
  Uniform,
  /// Storage buffer data, potentially mutable.
  Storage { writeable: bool },
  /// Opaque handles, such as samplers and images.
  Handle,
}

impl AddressSpace {
  pub const fn writeable(self) -> bool {
    match self {
      AddressSpace::Function => true,
      AddressSpace::Private => true,
      AddressSpace::WorkGroup => true,
      AddressSpace::Uniform => false,
      AddressSpace::Storage { writeable } => writeable,
      AddressSpace::Handle => false,
    }
  }
  pub const fn loadable(self) -> bool {
    !matches!(self, AddressSpace::Handle)
  }
}

pub struct Atomic<T>(PhantomData<T>);

impl<T: AtomicityShaderNodeType> ShaderNodeType for Atomic<T> {
  const TYPE: ShaderValueType = ShaderValueType::Single(ShaderValueSingleType::Sized(
    ShaderSizedValueType::Atomic(T::ATOM),
  ));
}

pub struct ShaderLocalPtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderPrivatePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderHandlePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderUniformPtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderReadOnlyStoragePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderStoragePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderWorkGroupPtr<T: ?Sized>(PhantomData<T>);

impl<T: ShaderNodeType> ShaderNodeType for ShaderLocalPtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderPrivatePtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderHandlePtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderUniformPtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}
impl<T: ShaderNodeType + ?Sized> ShaderNodeType for ShaderReadOnlyStoragePtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}
impl<T: ShaderNodeType + ?Sized> ShaderNodeType for ShaderStoragePtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderWorkGroupPtr<T> {
  const TYPE: ShaderValueType = T::TYPE;
}

pub type GlobalVarNode<T> = Node<ShaderPrivatePtr<T>>;
pub type LocalVarNode<T> = Node<ShaderLocalPtr<T>>;
pub type WorkGroupSharedNode<T> = Node<ShaderWorkGroupPtr<T>>;
pub type UniformNode<T> = Node<ShaderUniformPtr<T>>;
pub type HandleNode<T> = Node<ShaderHandlePtr<T>>;
pub type ReadOnlyStorageNode<T> = Node<ShaderReadOnlyStoragePtr<T>>;
pub type StorageNode<T> = Node<ShaderStoragePtr<T>>;

#[derive(Clone, Copy)]
pub struct BindingArray<T: ?Sized, const N: usize>(PhantomData<T>);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderAtomicValueType {
  I32,
  U32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderValueType {
  Single(ShaderValueSingleType),
  BindingArray {
    count: usize,
    ty: ShaderValueSingleType,
  },
  Never,
}
impl ShaderValueType {
  pub fn mutate_single<R>(
    &mut self,
    mut mutator: impl FnMut(&mut ShaderValueSingleType) -> R,
  ) -> Option<R> {
    match self {
      ShaderValueType::Single(v) => mutator(v).into(),
      ShaderValueType::BindingArray { ty, .. } => mutator(ty).into(),
      ShaderValueType::Never => None,
    }
  }
  pub fn visit_single<R>(&self, mut visitor: impl FnMut(&ShaderValueSingleType) -> R) -> Option<R> {
    match self {
      ShaderValueType::Single(v) => visitor(v).into(),
      ShaderValueType::BindingArray { ty, .. } => visitor(ty).into(),
      ShaderValueType::Never => None,
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderValueSingleType {
  Sized(ShaderSizedValueType),
  Unsized(ShaderUnSizedValueType),
  Sampler(SamplerBindingType),
  Texture {
    dimension: TextureViewDimension,
    sample_type: TextureSampleType,
  },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderSizedValueType {
  Atomic(ShaderAtomicValueType),
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  FixedSizeArray((&'static ShaderSizedValueType, usize)),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderUnSizedValueType {
  UnsizedArray(&'static ShaderSizedValueType),
  UnsizedStruct(&'static ShaderUnSizedStructMetaInfo),
}

pub trait ShaderNodeType: 'static {
  const TYPE: ShaderValueType;
}

pub trait ShaderNodeSingleType: 'static {
  const SINGLE_TYPE: ShaderValueSingleType;
}

pub trait ShaderSizedValueNodeType: ShaderNodeType {
  const MEMBER_TYPE: ShaderSizedValueType;
}

pub trait ShaderUnsizedValueNodeType: ShaderNodeType {
  const UNSIZED_TYPE: ShaderUnSizedValueType;
}

pub enum MaybeUnsizedValueType {
  Sized(ShaderSizedValueType),
  Unsized(ShaderUnSizedValueType),
}

impl<T: ShaderSizedValueNodeType> ShaderMaybeUnsizedValueNodeType for T {
  const MAYBE_UNSIZED_TYPE: MaybeUnsizedValueType = MaybeUnsizedValueType::Sized(Self::MEMBER_TYPE);
}

pub trait ShaderMaybeUnsizedValueNodeType: ShaderNodeType {
  const MAYBE_UNSIZED_TYPE: MaybeUnsizedValueType;
}

pub trait PrimitiveShaderNodeType: ShaderNodeType + Default {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

pub trait AtomicityShaderNodeType: ShaderNodeType {
  const ATOM: ShaderAtomicValueType;
}

pub trait ShaderStructuralNodeType: ShaderNodeType + Sized {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
  fn construct(instance: Self::Instance) -> Node<Self>;
}
pub type ENode<T> = <T as ShaderStructuralNodeType>::Instance;

pub trait ShaderUnsizedStructuralNodeType: ShaderNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderUnSizedStructMetaInfo;
}

#[macro_export]
macro_rules! sg_node_impl {
  ($ty: ty, $ty_value: expr) => {
    impl ShaderNodeSingleType for $ty {
      const SINGLE_TYPE: ShaderValueSingleType = $ty_value;
    }
    impl ShaderNodeType for $ty {
      const TYPE: ShaderValueType = ShaderValueType::Single(Self::SINGLE_TYPE);
    }
  };
}

impl ShaderNodeType for AnyType {
  const TYPE: ShaderValueType = ShaderValueType::Never;
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeSingleType for [T; N] {
  const SINGLE_TYPE: ShaderValueSingleType =
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray((&T::MEMBER_TYPE, N)));
}
impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeType for [T; N] {
  const TYPE: ShaderValueType = ShaderValueType::Single(Self::SINGLE_TYPE);
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeSingleType for Shader140Array<T, N> {
  const SINGLE_TYPE: ShaderValueSingleType =
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray((&T::MEMBER_TYPE, N)));
}
impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeType for Shader140Array<T, N> {
  const TYPE: ShaderValueType = ShaderValueType::Single(Self::SINGLE_TYPE);
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderSizedValueNodeType for [T; N] {
  const MEMBER_TYPE: ShaderSizedValueType =
    ShaderSizedValueType::FixedSizeArray((&T::MEMBER_TYPE, N));
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderSizedValueNodeType
  for Shader140Array<T, N>
{
  const MEMBER_TYPE: ShaderSizedValueType =
    ShaderSizedValueType::FixedSizeArray((&T::MEMBER_TYPE, N));
}

impl<T: ShaderNodeSingleType + ?Sized, const N: usize> ShaderNodeType
  for BindingArray<ShaderHandlePtr<T>, N>
{
  const TYPE: ShaderValueType = ShaderValueType::BindingArray {
    ty: T::SINGLE_TYPE,
    count: N,
  };
}

impl<T: ShaderSizedValueNodeType> ShaderNodeType for [T] {
  const TYPE: ShaderValueType =
    ShaderValueType::Single(ShaderValueSingleType::Unsized(Self::UNSIZED_TYPE));
}
impl<T: ShaderSizedValueNodeType> ShaderUnsizedValueNodeType for [T] {
  const UNSIZED_TYPE: ShaderUnSizedValueType =
    ShaderUnSizedValueType::UnsizedArray(&T::MEMBER_TYPE);
}
impl<T: ShaderSizedValueNodeType> ShaderMaybeUnsizedValueNodeType for [T] {
  const MAYBE_UNSIZED_TYPE: MaybeUnsizedValueType =
    MaybeUnsizedValueType::Unsized(Self::UNSIZED_TYPE);
}
