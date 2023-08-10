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
}

#[derive(Clone)]
pub struct ConstNode {
  pub data: PrimitiveShaderValue,
}

#[must_use]
pub fn val<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  v.into()
}

impl ShaderGraphNodeExpr {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    modify_graph(|graph| unsafe { graph.make_expression(self).into_node() })
  }
}
