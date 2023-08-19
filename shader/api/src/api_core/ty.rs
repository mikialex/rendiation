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

impl core::marker::ConstParamTy for AddressSpace {}

pub struct ShaderPtr<T, const S: AddressSpace>(PhantomData<T>);

impl<T: ShaderNodeType, const S: AddressSpace> ShaderNodeType for ShaderPtr<T, S> {
  const TYPE: ShaderValueType = T::TYPE;
}

// we do not have alias rule like rust in shader, so clone copy at will
impl<T, const S: AddressSpace> Clone for ShaderPtr<T, S> {
  fn clone(&self) -> Self {
    Self(self.0)
  }
}
impl<T, const S: AddressSpace> Copy for ShaderPtr<T, S> {}

pub type GlobalVariable<T> = Node<ShaderPtr<T, { AddressSpace::Private }>>;
pub type LocalVarNode<T> = Node<ShaderPtr<T, { AddressSpace::Function }>>;
pub type WorkGroupSharedNode<T> = Node<ShaderPtr<T, { AddressSpace::WorkGroup }>>;

pub type UniformPtr<T> = ShaderPtr<T, { AddressSpace::Uniform }>;
pub type UniformNode<T> = Node<UniformPtr<T>>;
pub type HandlePtr<T> = ShaderPtr<T, { AddressSpace::Handle }>;
pub type HandleNode<T> = Node<HandlePtr<T>>;

pub type ReadOnlyStoragePtr<T> = ShaderPtr<T, { AddressSpace::Storage { writeable: false } }>;
pub type ReadOnlyStorageNode<T> = Node<ReadOnlyStoragePtr<T>>;
pub type StoragePtr<T> = ShaderPtr<T, { AddressSpace::Storage { writeable: true } }>;
pub type StorageNode<T> = Node<StoragePtr<T>>;

#[derive(Clone, Copy)]
pub struct BindingArray<T, const N: usize>(PhantomData<T>);

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
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  FixedSizeArray((&'static ShaderSizedValueType, usize)),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderUnSizedValueType {
  UnsizedArray(&'static ShaderSizedValueType),
  UnsizedStruct(&'static ShaderUnSizedStructMetaInfo),
}

pub trait ShaderNodeType: 'static + Copy {
  const TYPE: ShaderValueType;
}

pub trait ShaderNodeSingleType: 'static + Copy {
  const SINGLE_TYPE: ShaderValueSingleType;
}

pub trait ShaderSizedValueNodeType: ShaderNodeType {
  const MEMBER_TYPE: ShaderSizedValueType;
}

pub trait ShaderUnsizedValueNodeType: ShaderNodeType {
  const UNSIZED_TYPE: ShaderUnSizedValueType;
}

pub trait PrimitiveShaderNodeType: ShaderNodeType + Default {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

pub trait ShaderStructuralNodeType: ShaderNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
  fn construct(instance: Self::Instance) -> Node<Self>;
}
pub type ENode<T> = <T as ShaderStructuralNodeType>::Instance;

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

impl<T: ShaderNodeSingleType, const N: usize> ShaderNodeType for BindingArray<T, N> {
  const TYPE: ShaderValueType = ShaderValueType::BindingArray {
    ty: T::SINGLE_TYPE,
    count: N,
  };
}
