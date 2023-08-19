use crate::*;

mod operator;
pub use operator::*;

mod structor;
pub use structor::*;

mod sampling;
pub use sampling::*;

mod primitive;
pub use primitive::*;

mod func;
pub use func::*;

mod func_built_in;
pub use func_built_in::*;

pub enum ValueKind {
  Uint,
  Int,
  Float,
  Bool,
}

pub trait ValueType {
  const KIND: ValueKind;
  const BYTE_WIDTH: u8;
}
impl ValueType for u32 {
  const KIND: ValueKind = ValueKind::Uint;
  const BYTE_WIDTH: u8 = 4;
}
impl ValueType for i32 {
  const KIND: ValueKind = ValueKind::Int;
  const BYTE_WIDTH: u8 = 4;
}
impl ValueType for f32 {
  const KIND: ValueKind = ValueKind::Float;
  const BYTE_WIDTH: u8 = 4;
}
impl ValueType for bool {
  const KIND: ValueKind = ValueKind::Bool;
  const BYTE_WIDTH: u8 = 1;
}

#[derive(Clone, Copy)]
pub enum SampleLevel {
  Auto,
  Zero,
  Exact(ShaderNodeRawHandle),
  Bias(ShaderNodeRawHandle),
  Gradient {
    x: ShaderNodeRawHandle,
    y: ShaderNodeRawHandle,
  },
}

pub enum ShaderNodeExpr {
  Zeroed {
    target: ShaderSizedValueType,
  },
  Convert {
    source: ShaderNodeRawHandle,
    convert_to: ValueKind,
    convert: Option<u8>,
  },
  FunctionCall {
    meta: ShaderFunctionType,
    parameters: Vec<ShaderNodeRawHandle>,
  },
  TextureSampling {
    texture: ShaderNodeRawHandle,
    sampler: ShaderNodeRawHandle,
    position: ShaderNodeRawHandle,
    index: Option<ShaderNodeRawHandle>,
    level: SampleLevel,
    reference: Option<ShaderNodeRawHandle>,
    offset: Option<Vec2<i32>>,
  },
  Swizzle {
    ty: &'static str,
    source: ShaderNodeRawHandle,
  },
  Derivative {
    axis: DerivativeAxis,
    ctrl: DerivativeControl,
    source: ShaderNodeRawHandle,
  },
  Compose {
    target: PrimitiveShaderValueType,
    parameters: Vec<ShaderNodeRawHandle>,
  },
  Operator(OperatorNode),
  FieldGet {
    field_index: usize,
    struct_node: ShaderNodeRawHandle,
  },
  StructConstruct {
    meta: &'static ShaderStructMetaInfo,
    fields: Vec<ShaderNodeRawHandle>,
  },
  Const {
    data: PrimitiveShaderValue,
  },
}

/// Hint at which precision to compute a derivative.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum DerivativeControl {
  Coarse,
  Fine,
  None,
}

/// Axis on which to compute a derivative.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum DerivativeAxis {
  X,
  Y,
  Width,
}

#[must_use]
pub fn val<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderNodeType,
{
  v.into()
}

#[must_use]
pub fn zeroed_val<T>() -> Node<T>
where
  T: ShaderSizedValueNodeType,
{
  ShaderNodeExpr::Zeroed {
    target: T::MEMBER_TYPE,
  }
  .insert_api()
}

impl ShaderNodeExpr {
  pub fn insert_api<T: ShaderNodeType>(self) -> Node<T> {
    call_shader_api(|api| unsafe { api.make_expression(self).into_node() })
  }
}
