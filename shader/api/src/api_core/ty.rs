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

#[repr(transparent)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct DeviceAtomic<T>(pub T);
unsafe impl<T: Std430> Std430 for DeviceAtomic<T> {
  const ALIGNMENT: usize = T::ALIGNMENT;
}

impl<T: AtomicityShaderNodeType> ShaderNodeType for DeviceAtomic<T> {
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Atomic(
      T::ATOM,
    )))
  }
}

impl<T> ShaderSizedValueNodeType for DeviceAtomic<T>
where
  T: AtomicityShaderNodeType + PrimitiveShaderNodeType,
{
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::Atomic(T::ATOM)
  }
  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Primitive(self.0.to_primitive())
  }
}

pub struct ShaderLocalPtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderPrivatePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderHandlePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderUniformPtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderReadOnlyStoragePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderStoragePtr<T: ?Sized>(PhantomData<T>);
pub struct ShaderWorkGroupPtr<T: ?Sized>(PhantomData<T>);

impl<T: ShaderNodeType> ShaderNodeType for ShaderLocalPtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderPrivatePtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderHandlePtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderUniformPtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}
impl<T: ShaderNodeType + ?Sized> ShaderNodeType for ShaderReadOnlyStoragePtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}
impl<T: ShaderNodeType + ?Sized> ShaderNodeType for ShaderStoragePtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}
impl<T: ShaderNodeType> ShaderNodeType for ShaderWorkGroupPtr<T> {
  fn ty() -> ShaderValueType {
    T::ty()
  }
}

pub type GlobalVarNode<T> = Node<ShaderPrivatePtr<T>>;
pub type LocalVarNode<T> = Node<ShaderLocalPtr<T>>;
pub type WorkGroupSharedNode<T> = Node<ShaderWorkGroupPtr<T>>;
pub type UniformNode<T> = Node<ShaderUniformPtr<T>>;
pub type HandleNode<T> = Node<ShaderHandlePtr<T>>;
pub type ReadOnlyStorageNode<T> = Node<ShaderReadOnlyStoragePtr<T>>;
pub type StorageNode<T> = Node<ShaderStoragePtr<T>>;

#[derive(Clone, Copy)]
pub struct BindingArray<T: ?Sized>(PhantomData<T>);

/// fixed size array in shader compile time, but dyn size in host runtime
#[derive(Clone, Copy)]
pub struct HostDynSizeArray<T>(PhantomData<T>);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderAtomicValueType {
  I32,
  U32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
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

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ShaderValueSingleType {
  Sized(ShaderSizedValueType),
  Unsized(ShaderUnSizedValueType),
  Sampler(SamplerBindingType),
  Texture {
    dimension: TextureViewDimension,
    sample_type: TextureSampleType,
    multi_sampled: bool,
  },
  StorageTexture {
    dimension: TextureViewDimension,
    format: StorageFormat,
    access: StorageTextureAccess,
  },
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderSizedValueType {
  Atomic(ShaderAtomicValueType),
  Primitive(PrimitiveShaderValueType),
  Struct(ShaderStructMetaInfo),
  FixedSizeArray(Box<ShaderSizedValueType>, usize),
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderUnSizedValueType {
  UnsizedArray(Box<ShaderSizedValueType>),
  UnsizedStruct(&'static ShaderUnSizedStructMetaInfo),
}

pub trait ShaderNodeType: 'static {
  fn ty() -> ShaderValueType;
}

pub trait ShaderNodeSingleType: 'static {
  fn single_ty() -> ShaderValueSingleType;
}

pub trait ShaderSizedValueNodeType: ShaderNodeType {
  fn sized_ty() -> ShaderSizedValueType;
  fn to_value(&self) -> ShaderStructFieldInitValue;
  fn to_shader_node_by_value(&self) -> Node<Self> {
    unsafe {
      self
        .to_value()
        .to_shader_node_by_value(&Self::sized_ty())
        .into_node()
    }
  }
}

#[derive(Clone)]
pub enum ShaderStructFieldInitValue {
  Primitive(PrimitiveShaderValue),
  Struct(Vec<ShaderStructFieldInitValue>),
  Array(Vec<ShaderStructFieldInitValue>),
}

impl ShaderStructFieldInitValue {
  pub fn to_shader_node_by_value(&self, ty: &ShaderSizedValueType) -> ShaderNodeRawHandle {
    match (self, ty) {
      (ShaderStructFieldInitValue::Primitive(v), ShaderSizedValueType::Primitive(_)) => {
        v.into_raw_node()
      }
      (ShaderStructFieldInitValue::Struct(parameters), ShaderSizedValueType::Struct(ty)) => {
        ShaderNodeExpr::Compose {
          target: ShaderSizedValueType::Struct(ty.clone()),
          parameters: parameters
            .iter()
            .zip(ty.fields.iter())
            .map(|(v, ty)| v.to_shader_node_by_value(&ty.ty))
            .collect(),
        }
        .insert_api::<AnyType>()
        .handle()
      }
      (
        ShaderStructFieldInitValue::Array(parameters),
        ShaderSizedValueType::FixedSizeArray(sty, _),
      ) => ShaderNodeExpr::Compose {
        target: ty.clone(),
        parameters: parameters
          .iter()
          .map(|v| v.to_shader_node_by_value(sty))
          .collect(),
      }
      .insert_api::<AnyType>()
      .handle(),
      _ => panic!("unexpected type miss match"),
    }
  }
}

pub trait ShaderUnsizedValueNodeType: ShaderNodeType {
  fn unsized_ty() -> ShaderUnSizedValueType;
}

pub enum MaybeUnsizedValueType {
  Sized(ShaderSizedValueType),
  Unsized(ShaderUnSizedValueType),
}

impl<T: ShaderSizedValueNodeType> ShaderMaybeUnsizedValueNodeType for T {
  fn maybe_unsized_ty() -> MaybeUnsizedValueType {
    MaybeUnsizedValueType::Sized(Self::sized_ty())
  }
}

pub trait ShaderMaybeUnsizedValueNodeType: ShaderNodeType {
  fn maybe_unsized_ty() -> MaybeUnsizedValueType;
}

pub trait PrimitiveShaderNodeType: ShaderNodeType + Default {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType;
  type Shape<T>;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

pub trait AtomicityShaderNodeType: ShaderNodeType {
  const ATOM: ShaderAtomicValueType;
}
impl AtomicityShaderNodeType for u32 {
  const ATOM: ShaderAtomicValueType = ShaderAtomicValueType::U32;
}
impl AtomicityShaderNodeType for i32 {
  const ATOM: ShaderAtomicValueType = ShaderAtomicValueType::I32;
}

pub trait ShaderStructuralNodeType: ShaderNodeType + Sized {
  type Instance;
  fn meta_info() -> ShaderStructMetaInfo;
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
      fn single_ty() -> ShaderValueSingleType {
        $ty_value
      }
    }
    impl ShaderNodeType for $ty {
      fn ty() -> ShaderValueType {
        ShaderValueType::Single(Self::single_ty())
      }
    }
  };
}

impl ShaderNodeType for AnyType {
  fn ty() -> ShaderValueType {
    ShaderValueType::Never
  }
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeSingleType for [T; N] {
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray(
      Box::new(T::sized_ty()),
      N,
    ))
  }
}
impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeType for [T; N] {
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeSingleType for Shader140Array<T, N> {
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray(
      Box::new(T::sized_ty()),
      N,
    ))
  }
}
impl<T: ShaderSizedValueNodeType, const N: usize> ShaderNodeType for Shader140Array<T, N> {
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderSizedValueNodeType for [T; N] {
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::FixedSizeArray(Box::new(T::sized_ty()), N)
  }

  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Array(self.iter().map(|x| x.to_value()).collect())
  }
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderSizedValueNodeType
  for Shader140Array<T, N>
{
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::FixedSizeArray(Box::new(T::sized_ty()), N)
  }
  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Array(self.inner.iter().map(|x| x.inner.to_value()).collect())
  }
}

impl<T: ShaderNodeSingleType + ?Sized> ShaderNodeType for BindingArray<ShaderHandlePtr<T>> {
  fn ty() -> ShaderValueType {
    ShaderValueType::BindingArray {
      ty: T::single_ty(),
      count: 0,
    }
  }
}
impl<T: ShaderNodeSingleType + ?Sized> ShaderNodeType for BindingArray<ShaderStoragePtr<T>> {
  fn ty() -> ShaderValueType {
    ShaderValueType::BindingArray {
      ty: T::single_ty(),
      count: 0,
    }
  }
}
impl<T: ShaderNodeSingleType + ?Sized> ShaderNodeType
  for BindingArray<ShaderReadOnlyStoragePtr<T>>
{
  fn ty() -> ShaderValueType {
    ShaderValueType::BindingArray {
      ty: T::single_ty(),
      count: 0,
    }
  }
}

impl<T: ShaderSizedValueNodeType> ShaderNodeType for [T] {
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

impl<T: ShaderSizedValueNodeType> ShaderNodeSingleType for [T] {
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedArray(
      Box::new(T::sized_ty()),
    ))
  }
}

impl<T: ShaderSizedValueNodeType> ShaderUnsizedValueNodeType for [T] {
  fn unsized_ty() -> ShaderUnSizedValueType {
    ShaderUnSizedValueType::UnsizedArray(Box::new(T::sized_ty()))
  }
}
impl<T: ShaderSizedValueNodeType> ShaderMaybeUnsizedValueNodeType for [T] {
  fn maybe_unsized_ty() -> MaybeUnsizedValueType {
    MaybeUnsizedValueType::Unsized(Self::unsized_ty())
  }
}

impl<T: ShaderSizedValueNodeType> ShaderNodeType for HostDynSizeArray<T> {
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

impl<T: ShaderSizedValueNodeType> ShaderNodeSingleType for HostDynSizeArray<T> {
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray(
      Box::new(T::sized_ty()),
      0,
    ))
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StorageTextureAccess {
  Load,
  Store,
  LoadStore,
}

/// Image storage format.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum StorageFormat {
  // 8-bit formats
  R8Unorm,
  R8Snorm,
  R8Uint,
  R8Sint,

  // 16-bit formats
  R16Uint,
  R16Sint,
  R16Float,
  Rg8Unorm,
  Rg8Snorm,
  Rg8Uint,
  Rg8Sint,

  // 32-bit formats
  R32Uint,
  R32Sint,
  R32Float,
  Rg16Uint,
  Rg16Sint,
  Rg16Float,
  Rgba8Unorm,
  Rgba8Snorm,
  Rgba8Uint,
  Rgba8Sint,
  Bgra8Unorm,

  // Packed 32-bit formats
  Rgb10a2Uint,
  Rgb10a2Unorm,

  // 64-bit formats
  Rg32Uint,
  Rg32Sint,
  Rg32Float,
  Rgba16Uint,
  Rgba16Sint,
  Rgba16Float,

  // 128-bit formats
  Rgba32Uint,
  Rgba32Sint,
  Rgba32Float,

  // Normalized 16-bit per channel formats
  R16Unorm,
  R16Snorm,
  Rg16Unorm,
  Rg16Snorm,
  Rgba16Unorm,
  Rgba16Snorm,
}

impl TryFrom<TextureFormat> for StorageFormat {
  type Error = ();

  fn try_from(value: TextureFormat) -> Result<Self, Self::Error> {
    Ok(match value {
      TextureFormat::R8Unorm => StorageFormat::R8Unorm,
      TextureFormat::R8Snorm => StorageFormat::R8Snorm,
      TextureFormat::R8Uint => StorageFormat::R8Uint,
      TextureFormat::R8Sint => StorageFormat::R8Sint,
      TextureFormat::R16Uint => StorageFormat::R16Uint,
      TextureFormat::R16Sint => StorageFormat::R16Sint,
      TextureFormat::R16Unorm => StorageFormat::R16Unorm,
      TextureFormat::R16Snorm => StorageFormat::R16Snorm,
      TextureFormat::R16Float => StorageFormat::R16Float,
      TextureFormat::Rg8Unorm => StorageFormat::Rg8Unorm,
      TextureFormat::Rg8Snorm => StorageFormat::Rg8Snorm,
      TextureFormat::Rg8Uint => StorageFormat::Rg8Uint,
      TextureFormat::Rg8Sint => StorageFormat::Rg8Sint,
      TextureFormat::R32Uint => StorageFormat::R32Uint,
      TextureFormat::R32Sint => StorageFormat::R32Sint,
      TextureFormat::R32Float => StorageFormat::R32Float,
      TextureFormat::Rg16Uint => StorageFormat::Rg16Uint,
      TextureFormat::Rg16Sint => StorageFormat::Rg16Sint,
      TextureFormat::Rg16Unorm => StorageFormat::Rg16Unorm,
      TextureFormat::Rg16Snorm => StorageFormat::Rg16Snorm,
      TextureFormat::Rg16Float => StorageFormat::Rg16Float,
      TextureFormat::Rgba8Unorm => StorageFormat::Rgba8Unorm,
      TextureFormat::Rgba8UnormSrgb => StorageFormat::Rgba8Unorm,
      TextureFormat::Rgba8Snorm => StorageFormat::Rgba8Snorm,
      TextureFormat::Rgba8Uint => StorageFormat::Rgba8Uint,
      TextureFormat::Rgba8Sint => StorageFormat::Rgba8Sint,
      TextureFormat::Bgra8Unorm => StorageFormat::Rgba8Unorm,
      TextureFormat::Bgra8UnormSrgb => StorageFormat::Rgba8Unorm,
      TextureFormat::Rgb10a2Uint => StorageFormat::Rgb10a2Uint,
      TextureFormat::Rgb10a2Unorm => StorageFormat::Rgb10a2Unorm,
      TextureFormat::Rg32Uint => StorageFormat::Rg32Uint,
      TextureFormat::Rg32Sint => StorageFormat::Rg32Sint,
      TextureFormat::Rg32Float => StorageFormat::Rg32Float,
      TextureFormat::Rgba16Uint => StorageFormat::Rgba16Uint,
      TextureFormat::Rgba16Sint => StorageFormat::Rgba16Sint,
      TextureFormat::Rgba16Unorm => StorageFormat::Rgba16Unorm,
      TextureFormat::Rgba16Snorm => StorageFormat::Rgba16Snorm,
      TextureFormat::Rgba16Float => StorageFormat::Rgba16Float,
      TextureFormat::Rgba32Uint => StorageFormat::Rgba32Uint,
      TextureFormat::Rgba32Sint => StorageFormat::Rgba32Sint,
      TextureFormat::Rgba32Float => StorageFormat::Rgba32Float,
      _ => return Err(()),
    })
  }
}
