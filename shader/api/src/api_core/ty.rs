use crate::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderStages {
  Vertex,
  Fragment,
}

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
