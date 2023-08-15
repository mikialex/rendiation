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

pub enum ShaderNodeExpr {
  FunctionCall {
    meta: ShaderFunctionType,
    parameters: Vec<ShaderNodeRawHandle>,
  },
  TextureSampling {
    texture: ShaderNodeRawHandle,
    sampler: ShaderNodeRawHandle,
    position: ShaderNodeRawHandle,
    index: Option<ShaderNodeRawHandle>,
    level: Option<ShaderNodeRawHandle>,
    reference: Option<ShaderNodeRawHandle>,
    offset: Option<Vec2<i32>>,
  },
  Swizzle {
    ty: &'static str,
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

#[must_use]
pub fn val<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderNodeType,
{
  v.into()
}

impl ShaderNodeExpr {
  pub fn insert_api<T: ShaderNodeType>(self) -> Node<T> {
    call_shader_api(|api| unsafe { api.make_expression(self).into_node() })
  }
}
