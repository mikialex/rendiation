use crate::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderStages {
  Vertex,
  Fragment,
}

#[derive(Clone, Copy)]
pub struct BindingArray<T, const N: usize>(PhantomData<T>);

#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderValueSingleType {
  Sized(ShaderSizedValueType),
  Unsized(ShaderUnSizedValueType),
  Sampler(SamplerBindingType),
  CompareSampler,
  Texture {
    dimension: TextureViewDimension,
    sample_type: TextureSampleType,
  },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderSizedValueType {
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  FixedSizeArray((&'static ShaderSizedValueType, usize)),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderUnSizedValueType {
  UnsizedArray(&'static ShaderSizedValueType),
  UnsizedStruct(&'static ShaderUnSizedStructMetaInfo),
}

pub trait ShaderGraphNodeType: 'static + Copy {
  const TYPE: ShaderValueType;
}

pub trait ShaderGraphNodeSingleType: 'static + Copy {
  const SINGLE_TYPE: ShaderValueSingleType;
}

pub trait ShaderSizedValueNodeType: ShaderGraphNodeType {
  const MEMBER_TYPE: ShaderSizedValueType;
}

pub trait ShaderUnsizedValueNodeType: ShaderGraphNodeType {
  const UNSIZED_TYPE: ShaderUnSizedValueType;
}

pub trait PrimitiveShaderGraphNodeType: ShaderGraphNodeType + Default {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

pub trait ShaderGraphStructuralNodeType: ShaderGraphNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
  fn construct(instance: Self::Instance) -> Node<Self>;
}
pub type ENode<T> = <T as ShaderGraphStructuralNodeType>::Instance;

#[macro_export]
macro_rules! sg_node_impl {
  ($ty: ty, $ty_value: expr) => {
    impl ShaderGraphNodeSingleType for $ty {
      const SINGLE_TYPE: ShaderValueSingleType = $ty_value;
    }
    impl ShaderGraphNodeType for $ty {
      const TYPE: ShaderValueType = ShaderValueType::Single(Self::SINGLE_TYPE);
    }
  };
}

impl ShaderGraphNodeType for AnyType {
  const TYPE: ShaderValueType = ShaderValueType::Never;
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderGraphNodeSingleType for [T; N] {
  const SINGLE_TYPE: ShaderValueSingleType =
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray((&T::MEMBER_TYPE, N)));
}
impl<T: ShaderSizedValueNodeType, const N: usize> ShaderGraphNodeType for [T; N] {
  const TYPE: ShaderValueType = ShaderValueType::Single(Self::SINGLE_TYPE);
}

impl<T: ShaderSizedValueNodeType, const N: usize> ShaderGraphNodeSingleType
  for Shader140Array<T, N>
{
  const SINGLE_TYPE: ShaderValueSingleType =
    ShaderValueSingleType::Sized(ShaderSizedValueType::FixedSizeArray((&T::MEMBER_TYPE, N)));
}
impl<T: ShaderSizedValueNodeType, const N: usize> ShaderGraphNodeType for Shader140Array<T, N> {
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

impl<T: ShaderGraphNodeSingleType, const N: usize> ShaderGraphNodeType for BindingArray<T, N> {
  const TYPE: ShaderValueType = ShaderValueType::BindingArray {
    ty: T::SINGLE_TYPE,
    count: N,
  };
}
