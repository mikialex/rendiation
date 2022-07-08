use crate::*;

pub enum ShaderGraphNodeExpr {
  FunctionCall {
    meta: &'static ShaderFunctionMetaInfo,
    parameters: Vec<ShaderGraphNodeRawHandle>,
  },
  TextureSampling {
    texture: ShaderGraphNodeRawHandle,
    sampler: ShaderGraphNodeRawHandle,
    position: ShaderGraphNodeRawHandle,
  },
  SamplerCombinedTextureSampling {
    texture: ShaderGraphNodeRawHandle,
    position: ShaderGraphNodeRawHandle,
  },
  Swizzle {
    ty: &'static str,
    source: ShaderGraphNodeRawHandle,
  },
  Compose {
    target: PrimitiveShaderValueType,
    parameters: Vec<ShaderGraphNodeRawHandle>,
  },
  MatInverse(ShaderGraphNodeRawHandle),
  MatTranspose(ShaderGraphNodeRawHandle),
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
  Write {
    source: ShaderGraphNodeRawHandle,
    target: ShaderGraphNodeRawHandle,
    /// implicit true is describe the write behavior
    /// of a scope to a value, the wrote value is a new
    /// value could be depend, so it's a new node.
    implicit: bool,
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
    source: ShaderIteratorAble,
    scope: ShaderGraphScope,
    iter: ShaderGraphNodeRawHandle,
  },
  // While,
}

pub enum ShaderIteratorAble {
  Const(u32),
  Count(Node<u32>),
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
    index: usize,
  },
  FragmentIn {
    ty: PrimitiveShaderValueType,
    index: usize,
  },
}

#[derive(Copy, Clone)]
pub enum ShaderBuiltIn {
  VertexIndexId,
  VertexInstanceId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderStages {
  Vertex,
  Fragment,
}

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
pub struct ShaderDepthTexture2DArray;
#[derive(Clone, Copy)]
pub struct ShaderDepthTextureCubeArray;
#[derive(Clone, Copy)]
pub struct ShaderDepthCube;

#[derive(Clone, Copy)]
pub struct ShaderSampler;
#[derive(Clone, Copy)]
pub struct ShaderCompareSampler;
#[derive(Clone, Copy)]
pub struct ShaderSamplerCombinedTexture;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimitiveShaderValueType {
  Bool,
  Int32,
  Uint32,
  Float32,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
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
  Float32(f32),
  Vec2Float32(Vec2<f32>),
  Vec3Float32(Vec3<f32>),
  Vec4Float32(Vec4<f32>),
  Mat2Float32(Mat2<f32>),
  Mat3Float32(Mat3<f32>),
  Mat4Float32(Mat4<f32>),
}

impl From<PrimitiveShaderValue> for PrimitiveShaderValueType {
  fn from(v: PrimitiveShaderValue) -> Self {
    match v {
      PrimitiveShaderValue::Uint32(_) => PrimitiveShaderValueType::Uint32,
      PrimitiveShaderValue::Float32(_) => PrimitiveShaderValueType::Float32,
      PrimitiveShaderValue::Vec2Float32(_) => PrimitiveShaderValueType::Vec2Float32,
      PrimitiveShaderValue::Vec3Float32(_) => PrimitiveShaderValueType::Vec3Float32,
      PrimitiveShaderValue::Vec4Float32(_) => PrimitiveShaderValueType::Vec4Float32,
      PrimitiveShaderValue::Mat2Float32(_) => PrimitiveShaderValueType::Mat2Float32,
      PrimitiveShaderValue::Mat3Float32(_) => PrimitiveShaderValueType::Mat3Float32,
      PrimitiveShaderValue::Mat4Float32(_) => PrimitiveShaderValueType::Mat4Float32,
      PrimitiveShaderValue::Bool(_) => PrimitiveShaderValueType::Bool,
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
  Fixed(ShaderStructMemberValueType),
  Sampler,
  CompareSampler,
  Texture {
    dimension: TextureViewDimension,
    sample_type: TextureSampleType,
  },
  SamplerCombinedTexture,
  Never,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderStructMemberValueType {
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  FixedSizeArray((&'static ShaderStructMemberValueType, usize)),
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

/// https://www.w3.org/TR/WGSL/#builtin-values
#[derive(Debug, Copy, Clone)]
pub enum ShaderBuiltInDecorator {
  VertexIndex,
  InstanceIndex,
  VertexPositionOut,
  FragmentPositionIn,
}

#[derive(Debug, Copy, Clone)]
pub enum ShaderFieldDecorator {
  BuiltIn(ShaderBuiltInDecorator),
  Location(usize),
}

pub trait ShaderFieldTypeMapper {
  type ShaderType: ShaderStructMemberValueNodeType;
}

impl<T: ShaderStructMemberValueNodeType> ShaderFieldTypeMapper for T {
  type ShaderType = T;
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
      ty: T::TYPE,
      ty_deco: None,
    });
    self
  }
}

pub trait ShaderGraphNodeType: 'static + Copy {
  const TYPE: ShaderValueType;
  fn extract_struct_define() -> Option<&'static ShaderStructMetaInfo> {
    match Self::TYPE {
      ShaderValueType::Fixed(v) => {
        if let ShaderStructMemberValueType::Struct(s) = v {
          Some(s)
        } else {
          None
        }
      }
      _ => None,
    }
  }
}

pub trait ShaderStructMemberValueNodeType {
  const TYPE: ShaderStructMemberValueType;
}

pub trait PrimitiveShaderGraphNodeType: ShaderGraphNodeType + Default {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

pub trait VertexInShaderGraphNodeType: PrimitiveShaderGraphNodeType {
  fn to_vertex_format() -> VertexFormat;
}

pub trait ShaderGraphStructuralNodeType: ShaderGraphNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
  fn construct(instance: Self::Instance) -> Node<Self>;
}
pub type ExpandedNode<T> = <T as ShaderGraphStructuralNodeType>::Instance;

#[derive(Copy, Clone)]
pub enum ShaderVaryingInterpolation {
  Flat,
  Perspective,
}

#[derive(Default, Clone)]
pub struct ShaderGraphBindGroup {
  pub bindings: Vec<(ShaderValueType, Rc<Cell<ShaderStageVisibility>>)>,
}

// use bitset
#[derive(Clone, Copy)]
pub enum ShaderStageVisibility {
  Vertex,
  Fragment,
  Both,
  None,
}

impl ShaderStageVisibility {
  pub fn is_visible_to(&self, stage: ShaderStages) -> bool {
    match self {
      ShaderStageVisibility::Vertex => {
        matches!(stage, ShaderStages::Vertex)
      }
      ShaderStageVisibility::Fragment => {
        matches!(stage, ShaderStages::Fragment)
      }
      ShaderStageVisibility::Both => true,
      ShaderStageVisibility::None => false,
    }
  }
}
