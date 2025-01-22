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

mod ray_query;
pub use ray_query::*;

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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
pub struct ShaderTextureLoad {
  pub texture: ShaderNodeRawHandle,
  pub position: ShaderNodeRawHandle,
  pub array_index: Option<ShaderNodeRawHandle>,
  pub sample_index: Option<ShaderNodeRawHandle>,
  pub level: Option<ShaderNodeRawHandle>,
}

#[derive(Clone, Copy)]
pub struct ShaderTextureStore {
  pub image: ShaderNodeRawHandle,
  pub position: ShaderNodeRawHandle,
  pub array_index: Option<ShaderNodeRawHandle>,
  pub value: ShaderNodeRawHandle,
}

// struct RayDesc {
//   flags: u32,
//   cull_mask: u32,
//   t_min: f32,
//   t_max: f32,
//   origin: vec3<f32>,
//   dir: vec3<f32>,
// }
#[derive(Clone, Copy)]
pub struct ShaderRayDesc {
  pub flags: ShaderNodeRawHandle,
  pub cull_mask: ShaderNodeRawHandle,
  pub t_min: ShaderNodeRawHandle,
  pub t_max: ShaderNodeRawHandle,
  pub origin: ShaderNodeRawHandle,
  pub dir: ShaderNodeRawHandle,
}

pub enum ShaderNodeExpr {
  Fake,
  Zeroed {
    target: ShaderSizedValueType,
  },
  Convert {
    source: ShaderNodeRawHandle,
    convert_to: ValueKind,
    /// this is channel wise byte size, if not specified, it will be bitcast conversion.
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
  RayQueryProceed {
    ray_query: ShaderNodeRawHandle,
  },
  RayQueryGetCandidateIntersection {
    ray_query: ShaderNodeRawHandle,
  },
  RayQueryGetCommitedIntersection {
    ray_query: ShaderNodeRawHandle,
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
    weak: bool,
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

/// # Safety
///
/// the upper layer may create "fake" node for error handling downgrade purpose.
/// the api's backend implementation could do anything to return a fake node.
#[must_use]
pub unsafe fn fake_val<T: ShaderNodeType>() -> Node<T> {
  ShaderNodeExpr::Fake.insert_api()
}

impl ShaderNodeExpr {
  pub fn insert_api<T: ShaderNodeType>(self) -> Node<T> {
    call_shader_api(|api| unsafe { api.make_expression(self).into_node() })
  }
}
