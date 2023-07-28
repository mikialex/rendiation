use crate::*;

pub enum ShaderBuiltInFunction {
  MatTranspose,
  Normalize,
  Length,
  Dot,
  Cross,
  SmoothStep,
  Select,
  Min,
  Max,
  Clamp,
  Abs,
  Pow,
  Saturate,
  // todo other math
}

pub enum ShaderFunctionType {
  Custom(&'static ShaderFunctionMetaInfo),
  BuiltIn(ShaderBuiltInFunction),
}

pub enum ShaderGraphNodeExpr {
  FunctionCall {
    meta: ShaderFunctionType,
    parameters: Vec<ShaderGraphNodeRawHandle>,
  },
  TextureSampling {
    texture: ShaderGraphNodeRawHandle,
    sampler: ShaderGraphNodeRawHandle,
    position: ShaderGraphNodeRawHandle,
    index: Option<ShaderGraphNodeRawHandle>,
    level: Option<ShaderGraphNodeRawHandle>,
  },
  Swizzle {
    ty: &'static str,
    source: ShaderGraphNodeRawHandle,
  },
  Compose {
    target: PrimitiveShaderValueType,
    parameters: Vec<ShaderGraphNodeRawHandle>,
  },
  MatShrink {
    source: ShaderGraphNodeRawHandle,
    dimension: usize,
  },
  Operator(OperatorNode),
  FieldGet {
    field_name: &'static str,
    struct_node: ShaderGraphNodeRawHandle,
  },
  StructConstruct {
    meta: &'static ShaderStructMetaInfo,
    fields: Vec<ShaderGraphNodeRawHandle>,
  },
  Const(ConstNode),
  Copy(ShaderGraphNodeRawHandle),
}

pub struct ShaderGraphNodeData {
  pub node: ShaderGraphNode,
  pub ty: ShaderValueType,
}

pub enum ShaderGraphNode {
  Input(ShaderGraphInputNode),
  /// This is workaround for some case
  UnNamed,
  /// the old maybe not exist, but we require a side effect write
  Write {
    new: ShaderGraphNodeRawHandle,
    old: Option<ShaderGraphNodeRawHandle>,
  },
  ControlFlow(ShaderControlFlowNode),
  SideEffect(ShaderSideEffectNode),
  Expr(ShaderGraphNodeExpr),
}

pub enum ShaderSideEffectNode {
  Continue,
  Break,
  Return(ShaderGraphNodeRawHandle),
  Termination,
}

pub enum ShaderControlFlowNode {
  If {
    condition: ShaderGraphNodeRawHandle,
    scope: ShaderGraphScope,
  },
  For {
    source: ShaderIterator,
    scope: ShaderGraphScope,
    index: ShaderGraphNodeRawHandle,
    iter: ShaderGraphNodeRawHandle,
  },
}

pub trait ShaderIteratorAble {
  type Item: ShaderGraphNodeType;
}

pub enum ShaderIterator {
  Const(u32),
  Count(ShaderGraphNodeRawHandle),
  FixedArray {
    array: ShaderGraphNodeRawHandle,
    length: usize,
  },
  Clamped {
    source: Box<Self>,
    max: ShaderGraphNodeRawHandle,
  },
}

#[derive(Clone)]
pub struct ConstNode {
  pub data: PrimitiveShaderValue,
}

pub enum UnaryOperator {
  LogicalNot,
}

pub enum BinaryOperator {
  Add,
  Sub,
  Mul,
  Div,
  Rem,
  Eq,
  NotEq,
  GreaterThan,
  LessThan,
  GreaterEqualThan,
  LessEqualThan,
  LogicalOr,
  LogicalAnd,
}
pub enum OperatorNode {
  Unary {
    one: ShaderGraphNodeRawHandle,
    operator: UnaryOperator,
  },
  Binary {
    left: ShaderGraphNodeRawHandle,
    right: ShaderGraphNodeRawHandle,
    operator: BinaryOperator,
  },
  Index {
    array: ShaderGraphNodeRawHandle,
    entry: ShaderGraphNodeRawHandle,
  },
}

#[derive(Clone)]
pub enum ShaderGraphInputNode {
  BuiltIn(ShaderBuiltIn),
  Uniform {
    bindgroup_index: usize,
    entry_index: usize,
  },
  VertexIn {
    ty: PrimitiveShaderValueType,
    location: usize,
  },
  FragmentIn {
    ty: PrimitiveShaderValueType,
    location: usize,
  },
}

#[derive(Copy, Clone)]
pub enum ShaderBuiltIn {
  VertexIndexId,
  VertexInstanceId,
  FragmentFrontFacing,
  FragmentSampleIndex,
  FragmentSampleMask,
  FragmentNDC,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderStages {
  Vertex,
  Fragment,
}

#[derive(Clone, Copy)]
pub struct BindingArray<T, const N: usize>(PhantomData<T>);

#[derive(Clone, Copy)]
pub struct ShaderTexture1D;
#[derive(Clone, Copy)]
pub struct ShaderTexture2D;
#[derive(Clone, Copy)]
pub struct ShaderTexture3D;
#[derive(Clone, Copy)]
pub struct ShaderTextureCube;
#[derive(Clone, Copy)]
pub struct ShaderTexture2DArray;
#[derive(Clone, Copy)]
pub struct ShaderTextureCubeArray;
#[derive(Clone, Copy)]
pub struct ShaderDepthTexture2D;
#[derive(Clone, Copy)]
pub struct ShaderDepthTextureCube;
#[derive(Clone, Copy)]
pub struct ShaderDepthTexture2DArray;
#[derive(Clone, Copy)]
pub struct ShaderDepthTextureCubeArray;

#[derive(Clone, Copy)]
pub struct ShaderSampler;
#[derive(Clone, Copy)]
pub struct ShaderCompareSampler;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimitiveShaderValueType {
  Bool,
  Int32,
  Uint32,
  Float32,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
  Vec2Uint32,
  Vec3Uint32,
  Vec4Uint32,
  Mat2Float32,
  Mat3Float32,
  Mat4Float32,
}

pub enum PrimitiveScalarShaderType {
  Int32,
  Uint32,
  Float32,
}

#[derive(Clone, Copy)]
pub enum PrimitiveShaderValue {
  Bool(bool),
  Uint32(u32),
  Int32(i32),
  Float32(f32),
  Vec2Float32(Vec2<f32>),
  Vec3Float32(Vec3<f32>),
  Vec4Float32(Vec4<f32>),
  Vec2Uint32(Vec2<u32>),
  Vec3Uint32(Vec3<u32>),
  Vec4Uint32(Vec4<u32>),
  Mat2Float32(Mat2<f32>),
  Mat3Float32(Mat3<f32>),
  Mat4Float32(Mat4<f32>),
}

impl From<PrimitiveShaderValue> for PrimitiveShaderValueType {
  fn from(v: PrimitiveShaderValue) -> Self {
    match v {
      PrimitiveShaderValue::Int32(_) => PrimitiveShaderValueType::Int32,
      PrimitiveShaderValue::Uint32(_) => PrimitiveShaderValueType::Uint32,
      PrimitiveShaderValue::Float32(_) => PrimitiveShaderValueType::Float32,
      PrimitiveShaderValue::Vec2Float32(_) => PrimitiveShaderValueType::Vec2Float32,
      PrimitiveShaderValue::Vec3Float32(_) => PrimitiveShaderValueType::Vec3Float32,
      PrimitiveShaderValue::Vec4Float32(_) => PrimitiveShaderValueType::Vec4Float32,
      PrimitiveShaderValue::Mat2Float32(_) => PrimitiveShaderValueType::Mat2Float32,
      PrimitiveShaderValue::Mat3Float32(_) => PrimitiveShaderValueType::Mat3Float32,
      PrimitiveShaderValue::Mat4Float32(_) => PrimitiveShaderValueType::Mat4Float32,
      PrimitiveShaderValue::Bool(_) => PrimitiveShaderValueType::Bool,
      PrimitiveShaderValue::Vec2Uint32(_) => PrimitiveShaderValueType::Vec2Uint32,
      PrimitiveShaderValue::Vec3Uint32(_) => PrimitiveShaderValueType::Vec3Uint32,
      PrimitiveShaderValue::Vec4Uint32(_) => PrimitiveShaderValueType::Vec4Uint32,
    }
  }
}

/// use for compile time ubo field reflection by procedure macro;
#[derive(Debug)]
pub struct ShaderFunctionMetaInfo {
  pub function_name: &'static str,
  pub function_source: &'static str,
  pub depend_functions: &'static [&'static ShaderFunctionMetaInfo],
  pub depend_types: &'static [&'static ShaderStructMetaInfo],
}

// todo use other uuid mechanism as identity
impl Eq for ShaderFunctionMetaInfo {}
impl PartialEq for ShaderFunctionMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.function_name == other.function_name
  }
}

impl Hash for ShaderFunctionMetaInfo {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.function_name.hash(state);
  }
}

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
  Fixed(ShaderStructMemberValueType),
  Unsized(ShaderUnSizedValueType),
  Sampler(SamplerBindingType),
  CompareSampler,
  Texture {
    dimension: TextureViewDimension,
    sample_type: TextureSampleType,
  },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderStructMemberValueType {
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  FixedSizeArray((&'static ShaderStructMemberValueType, usize)),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderUnSizedValueType {
  UnsizedArray(&'static ShaderStructMemberValueType),
  UnsizedStruct(&'static ShaderUnSizedStructMetaInfo),
}

/// use for compile time ubo field reflection by procedure macro;
#[derive(Debug)]
pub struct ShaderStructMetaInfo {
  pub name: &'static str,
  pub fields: &'static [ShaderStructFieldMetaInfo],
}

impl ShaderStructMetaInfo {
  pub fn to_owned(&self) -> ShaderStructMetaInfoOwned {
    ShaderStructMetaInfoOwned {
      name: self.name.to_owned(),
      fields: self.fields.iter().map(|f| f.to_owned()).collect(),
    }
  }
}

impl PartialEq for ShaderStructMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}
impl Eq for ShaderStructMetaInfo {}
impl Hash for ShaderStructMetaInfo {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

#[derive(Debug)]
pub struct ShaderUnSizedStructMetaInfo {
  pub name: &'static str,
  pub sized_fields: &'static [ShaderStructFieldMetaInfo],
  /// according to spec, only unsized array is supported, unsized struct is not
  ///
  /// https://www.w3.org/TR/WGSL/#struct-types
  pub last_dynamic_array_field: (&'static str, &'static ShaderStructMemberValueType),
}

impl PartialEq for ShaderUnSizedStructMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}
impl Eq for ShaderUnSizedStructMetaInfo {}
impl Hash for ShaderUnSizedStructMetaInfo {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

/// https://www.w3.org/TR/WGSL/#builtin-values
#[derive(Debug, Copy, Clone)]
pub enum ShaderBuiltInDecorator {
  VertexIndex,
  InstanceIndex,
  VertexPositionOut,
  FragmentPositionIn,
  FrontFacing,
  FragDepth,
}

#[derive(Debug, Copy, Clone)]
pub enum ShaderFieldDecorator {
  BuiltIn(ShaderBuiltInDecorator),
  Location(usize),
}

/// This trait is to mapping the real struct ty into the shadergraph node ty.
/// These types may be different because the std140 type substitution
pub trait ShaderFieldTypeMapper {
  type ShaderType: ShaderStructMemberValueNodeType;
}

// Impl notes:
//
// impl<T: ShaderStructMemberValueNodeType> ShaderFieldTypeMapper for T {
//   type ShaderType = T;
// }
//
// The reason we can not use this(above) with default ShaderType specialization is
//  the compiler can't infer this type equality:
// `let v: <rendiation_algebra::Vec4<f32> as ShaderFieldTypeMapper>::ShaderType = Vec4::default();`
//
//  So we have to impl for all the types we know

macro_rules! shader_field_ty_mapper {
  ($src:ty, $dst:ty) => {
    impl ShaderFieldTypeMapper for $src {
      type ShaderType = $dst;
    }
  };
}

// standard
shader_field_ty_mapper!(f32, Self);
shader_field_ty_mapper!(u32, Self);
shader_field_ty_mapper!(i32, Self);
shader_field_ty_mapper!(Vec2<f32>, Self);
shader_field_ty_mapper!(Vec3<f32>, Self);
shader_field_ty_mapper!(Vec4<f32>, Self);
shader_field_ty_mapper!(Vec2<u32>, Self);
shader_field_ty_mapper!(Vec3<u32>, Self);
shader_field_ty_mapper!(Vec4<u32>, Self);
shader_field_ty_mapper!(Mat2<f32>, Self);
shader_field_ty_mapper!(Mat3<f32>, Self);
shader_field_ty_mapper!(Mat4<f32>, Self);

// std140
shader_field_ty_mapper!(Shader16PaddedMat2, Mat2<f32>);
shader_field_ty_mapper!(Shader16PaddedMat3, Mat3<f32>);
shader_field_ty_mapper!(Bool, bool);

impl<T: ShaderStructMemberValueNodeType, const U: usize> ShaderFieldTypeMapper
  for Shader140Array<T, U>
{
  type ShaderType = [T; U];
}

#[derive(Debug)]
pub struct ShaderStructFieldMetaInfo {
  pub name: &'static str,
  pub ty: ShaderStructMemberValueType,
  pub ty_deco: Option<ShaderFieldDecorator>,
}

impl ShaderStructFieldMetaInfo {
  pub fn to_owned(&self) -> ShaderStructFieldMetaInfoOwned {
    ShaderStructFieldMetaInfoOwned {
      name: self.name.to_owned(),
      ty: self.ty,
      ty_deco: self.ty_deco,
    }
  }
}

pub struct ShaderStructFieldMetaInfoOwned {
  pub name: String,
  pub ty: ShaderStructMemberValueType,
  pub ty_deco: Option<ShaderFieldDecorator>,
}

pub struct ShaderStructMetaInfoOwned {
  pub name: String,
  pub fields: Vec<ShaderStructFieldMetaInfoOwned>,
}

impl ShaderStructMetaInfoOwned {
  pub fn new(name: &str) -> Self {
    Self {
      name: name.to_owned(),
      fields: Default::default(),
    }
  }

  #[must_use]
  pub fn add_field<T: ShaderStructMemberValueNodeType>(mut self, name: &str) -> Self {
    self.fields.push(ShaderStructFieldMetaInfoOwned {
      name: name.to_owned(),
      ty: T::MEMBER_TYPE,
      ty_deco: None,
    });
    self
  }
}

pub trait ShaderGraphNodeType: 'static + Copy {
  const TYPE: ShaderValueType;
}

pub trait ShaderGraphNodeSingleType: 'static + Copy {
  const SINGLE_TYPE: ShaderValueSingleType;
}

pub trait ShaderStructMemberValueNodeType: ShaderGraphNodeType {
  const MEMBER_TYPE: ShaderStructMemberValueType;
}

pub trait ShaderUnsizedValueNodeType: ShaderGraphNodeType {
  const UNSIZED_TYPE: ShaderUnSizedValueType;
}

pub trait PrimitiveShaderGraphNodeType: ShaderGraphNodeType + Default {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

/// Mark self type could use as vertex buffer input
pub trait VertexInShaderGraphNodeType: PrimitiveShaderGraphNodeType {
  fn to_vertex_format() -> VertexFormat;
}

pub trait ShaderGraphStructuralNodeType: ShaderGraphNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
  fn construct(instance: Self::Instance) -> Node<Self>;
}
pub type ENode<T> = <T as ShaderGraphStructuralNodeType>::Instance;

#[derive(Copy, Clone)]
pub enum ShaderVaryingInterpolation {
  Flat,
  Perspective,
}

#[derive(Default, Clone)]
pub struct ShaderGraphBindGroup {
  pub bindings: Vec<ShaderGraphBindEntry>,
}

#[derive(Clone, Copy)]
pub struct ShaderGraphBindEntry {
  pub desc: ShaderBindingDescriptor,
  pub vertex_node: ShaderGraphNodeRawHandle,
  pub fragment_node: ShaderGraphNodeRawHandle,
}
