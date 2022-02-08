use std::any::TypeId;
use std::{
  collections::HashSet,
  hash::{Hash, Hasher},
};

pub use crate::*;

pub enum ShaderGraphNodeExpr {
  FunctionCall {
    prototype: &'static ShaderFunctionMetaInfo,
    parameters: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  TextureSampling(TextureSamplingNode),
  Swizzle {
    ty: &'static str,
    source: ShaderGraphNodeRawHandleUntyped,
  },
  Compose {
    target: PrimitiveShaderValueType,
    parameters: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  Operator(OperatorNode),
  FieldGet {
    field_name: &'static str,
    struct_node: ShaderGraphNodeRawHandleUntyped,
  },
  StructConstruct {
    struct_id: TypeId,
    fields: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  Const(ConstNode),
  Copy(ShaderGraphNodeRawHandleUntyped),
}

pub enum ShaderGraphNodeData {
  Input(ShaderGraphInputNode),
  /// This is workaround for some case
  UnNamed,
  Write {
    source: ShaderGraphNodeRawHandleUntyped,
    target: ShaderGraphNodeRawHandleUntyped,
    implicit: bool,
  },
  ControlFlow(ShaderControlFlowNode),
  SideEffect(ShaderSideEffectNode),
  Expr(ShaderGraphNodeExpr),
}

pub enum ShaderSideEffectNode {
  Continue,
  Break,
  Return(ShaderGraphNodeRawHandleUntyped),
  Termination,
}

pub enum ShaderControlFlowNode {
  If {
    condition: ShaderGraphNodeRawHandleUntyped,
    scope: ShaderGraphScope,
  },
  For {
    source: ShaderIteratorAble,
    scope: ShaderGraphScope,
  },
  // While,
}

#[derive(Clone)]
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
  pub texture: ShaderGraphNodeRawHandle<ShaderTexture>,
  pub sampler: ShaderGraphNodeRawHandle<ShaderSampler>,
  pub position: ShaderGraphNodeRawHandle<Vec2<f32>>,
}

#[derive(Clone)]
pub struct OperatorNode {
  pub left: ShaderGraphNodeRawHandleUntyped,
  pub right: ShaderGraphNodeRawHandleUntyped,
  pub operator: &'static str,
}

pub enum UnaryOperator {
  Not,
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
}

pub enum TrinaryOperator {
  IfElse,
}

pub enum OperatorNode2 {
  Unary {
    one: ShaderGraphNodeRawHandleUntyped,
    operator: &'static str,
  },
  Binary {
    left: ShaderGraphNodeRawHandleUntyped,
    right: ShaderGraphNodeRawHandleUntyped,
    operator: &'static str,
  },
  Trinary {
    forward: ShaderGraphNodeRawHandleUntyped,
    left: ShaderGraphNodeRawHandleUntyped,
    right: ShaderGraphNodeRawHandleUntyped,
    operator: &'static str,
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

// todo
#[derive(Copy, Clone)]
pub enum ShaderGraphVertexFragmentIOType {
  Float,
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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
pub enum ShaderValueType {
  Fixed(ShaderStructMemberValueType),
  Sampler,
  Texture,
  Never,
}

#[derive(Clone, Copy)]
pub enum ShaderStructMemberValueType {
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  // FixedSizeArray((&'static ShaderValueType, usize)),
}

/// use for compile time ubo field reflection by procedure macro;
pub struct ShaderStructMetaInfo {
  pub name: &'static str,
  pub fields: Vec<(&'static str, ShaderStructMemberValueType)>,
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
    self.fields.push((name, T::to_type()));
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

pub trait ShaderGraphStructuralNodeType: ShaderGraphNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
}
