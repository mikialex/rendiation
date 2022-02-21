use crate::*;

pub enum ShaderGraphNodeExpr {
  FunctionCall {
    meta: &'static ShaderFunctionMetaInfo,
    parameters: Vec<ShaderGraphNodeRawHandle>,
  },
  TextureSampling(TextureSamplingNode),
  Swizzle {
    ty: &'static str,
    source: ShaderGraphNodeRawHandle,
  },
  Compose {
    target: PrimitiveShaderValueType,
    parameters: Vec<ShaderGraphNodeRawHandle>,
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

pub enum ShaderGraphNodeData {
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

#[derive(Clone)]
pub struct TextureSamplingNode {
  pub texture: ShaderGraphNodeRawHandle,
  pub sampler: ShaderGraphNodeRawHandle,
  pub position: ShaderGraphNodeRawHandle,
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
pub struct ShaderTexture;
#[derive(Clone, Copy)]
pub struct ShaderSampler;

#[derive(Clone, Copy, PartialEq)]
pub enum PrimitiveShaderValueType {
  Bool,
  Uint32,
  Float32,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
  Mat2Float32,
  Mat3Float32,
  Mat4Float32,
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
#[derive(Debug, Eq)]
pub struct ShaderFunctionMetaInfo {
  pub function_name: &'static str,
  pub function_source: Option<&'static str>, // None is builtin function, no need to gen code
  pub depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
}

impl ShaderFunctionMetaInfo {
  #[must_use]
  pub fn declare_function_dep(mut self, f: &'static ShaderFunctionMetaInfo) -> Self {
    self.depend_functions.insert(f);
    self
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

impl PartialEq for ShaderFunctionMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.function_name == other.function_name
  }
}

impl ShaderFunctionMetaInfo {
  pub fn new(function_name: &'static str, function_source: Option<&'static str>) -> Self {
    Self {
      function_name,
      function_source,
      depend_functions: HashSet::new(),
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ShaderValueType {
  Fixed(ShaderStructMemberValueType),
  Sampler,
  Texture,
  Never,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ShaderStructMemberValueType {
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  // FixedSizeArray((&'static ShaderValueType, usize)),
}

/// use for compile time ubo field reflection by procedure macro;
pub struct ShaderStructMetaInfo {
  pub name: &'static str,
  pub fields: Vec<ShaderStructFieldMetaInfo>,
}

impl PartialEq for ShaderStructMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

/// https://www.w3.org/TR/WGSL/#builtin-values
pub enum ShaderBuiltInDecorator {
  VertexIndex,
  InstanceIndex,
  VertexPositionOut,
  FragmentPositionIn,
}

pub enum ShaderFieldDecorator {
  BuiltIn(ShaderBuiltInDecorator),
  Location(usize),
}

pub struct ShaderStructFieldMetaInfo {
  pub name: std::borrow::Cow<'static, str>,
  pub ty: ShaderStructMemberValueType,
  pub ty_deco: Option<ShaderFieldDecorator>,
}

impl ShaderStructMetaInfo {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      fields: Default::default(),
    }
  }

  #[must_use]
  pub fn add_field<T: ShaderStructMemberValueNodeType>(mut self, name: &'static str) -> Self {
    self.fields.push(ShaderStructFieldMetaInfo {
      name: std::borrow::Cow::Borrowed(name),
      ty: T::to_type(),
      ty_deco: None,
    });
    self
  }
}

pub trait ShaderGraphNodeType: 'static + Copy {
  fn to_type() -> ShaderValueType;
  fn extract_struct_define() -> Option<&'static ShaderStructMetaInfo> {
    match Self::to_type() {
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
  fn to_type() -> ShaderStructMemberValueType;
}

pub trait PrimitiveShaderGraphNodeType: ShaderGraphNodeType {
  fn to_primitive_type() -> PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

pub trait VertexInShaderGraphNodeType: PrimitiveShaderGraphNodeType {
  fn to_vertex_format() -> VertexFormat;
}

pub trait ShaderGraphStructuralNodeType: ShaderGraphNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
}

#[derive(Copy, Clone)]
pub enum ShaderVaryingInterpolation {
  Flat,
  Perspective,
}

#[derive(Default, Clone)]
pub struct ShaderGraphBindGroup {
  pub bindings: Vec<(ShaderValueType, Rc<Cell<ShaderStageVisibility>>)>,
}

#[derive(Clone)]
pub enum ShaderStageVisibility {
  Vertex,
  Fragment,
  Both,
  None,
}
