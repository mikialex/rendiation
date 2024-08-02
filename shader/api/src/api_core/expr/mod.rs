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

#[derive(Clone, Copy)]
pub enum TextureQuery {
  /// Get the size at the specified level.
  Size {
    /// If `None`, the base level is considered.
    level: Option<ShaderNodeRawHandle>,
  },
  /// Get the number of mipmap levels.
  NumLevels,
  /// Get the number of array layers.
  NumLayers,
  /// Get the number of samples.
  NumSamples,
}

#[derive(Clone, Copy)]
pub enum GatherChannel {
  X,
  Y,
  Z,
  W,
}

pub struct ShaderTextureSampling {
  pub texture: ShaderNodeRawHandle,
  pub sampler: ShaderNodeRawHandle,
  pub position: ShaderNodeRawHandle,
  pub array_index: Option<ShaderNodeRawHandle>,
  pub level: SampleLevel,
  pub reference: Option<ShaderNodeRawHandle>,
  pub offset: Option<Vec2<i32>>,
  pub gather_channel: Option<GatherChannel>,
}

pub struct ShaderTextureLoad {
  pub texture: ShaderNodeRawHandle,
  pub position: ShaderNodeRawHandle,
  pub array_index: Option<ShaderNodeRawHandle>,
  pub sample_index: Option<ShaderNodeRawHandle>,
  pub level: Option<ShaderNodeRawHandle>,
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
  AtomicCall {
    ty: ShaderAtomicValueType,
    pointer: ShaderNodeRawHandle,
    function: AtomicFunction,
    value: ShaderNodeRawHandle,
  },
  FunctionCall {
    meta: ShaderFunctionType,
    parameters: Vec<ShaderNodeRawHandle>,
  },
  TextureSampling(ShaderTextureSampling),
  TextureLoad(ShaderTextureLoad),
  TextureQuery(ShaderNodeRawHandle, TextureQuery),
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
    target: ShaderSizedValueType,
    parameters: Vec<ShaderNodeRawHandle>,
  },
  Operator(OperatorNode),
  IndexStatic {
    field_index: usize,
    target: ShaderNodeRawHandle,
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

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum AtomicFunction {
  Add,
  Subtract,
  And,
  ExclusiveOr,
  InclusiveOr,
  Min,
  Max,
  Exchange {
    compare: Option<ShaderNodeRawHandle>,
  },
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
    target: T::sized_ty(),
  }
  .insert_api()
}

impl ShaderNodeExpr {
  pub fn insert_api<T: ShaderNodeType>(self) -> Node<T> {
    call_shader_api(|api| unsafe { api.make_expression(self).into_node() })
  }
}
