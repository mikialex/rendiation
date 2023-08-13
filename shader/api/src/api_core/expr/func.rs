use crate::*;

pub enum ShaderFunctionType {
  Custom(ShaderUserDefinedFunction),
  BuiltIn(ShaderBuiltInFunction),
}

#[derive(Clone)]
pub struct ShaderUserDefinedFunction {
  name: String,
}

pub struct FunctionBuildCtx<T>(PhantomData<T>);

pub enum ShaderFnTryDefineResult<T> {
  NotDefined(FunctionBuildCtx<T>),
  AlreadyDefined(ShaderUserDefinedFunction),
}

impl<T: ShaderGraphNodeType> ShaderFnTryDefineResult<T> {
  pub fn or_define(self, f: impl FnOnce(&FunctionBuildCtx<T>)) -> ShaderUserDefinedFunction {
    match self {
      ShaderFnTryDefineResult::NotDefined(builder) => {
        f(&builder);
        builder.end_fn_define()
      }
      ShaderFnTryDefineResult::AlreadyDefined(meta) => meta,
    }
  }
}

// todo check T match returned meta
pub fn get_shader_fn<T: ShaderGraphNodeType>(name: String) -> ShaderFnTryDefineResult<T> {
  let info = modify_graph(|g| g.get_fn(name.clone()));

  match info {
    Some(info) => ShaderFnTryDefineResult::AlreadyDefined(info),
    None => ShaderFnTryDefineResult::NotDefined(FunctionBuildCtx::begin(name)),
  }
}

impl<T: ShaderGraphNodeType> FunctionBuildCtx<T> {
  pub fn begin(name: String) -> Self {
    let ty = T::TYPE;
    let ty = match ty {
      ShaderValueType::Never => None,
      _ => Some(ty),
    };
    modify_graph(|g| g.begin_define_fn(name, ty));
    Self(Default::default())
  }

  pub fn push_fn_parameter<P: ShaderGraphNodeType>(&self) -> Node<P> {
    unsafe { modify_graph(|g| g.push_fn_parameter(P::TYPE)).into_node() }
  }

  pub fn do_return(&self, r: Node<T>) {
    let handle = match T::TYPE {
      ShaderValueType::Never => None,
      _ => Some(r.handle()),
    };
    modify_graph(|g| g.do_return(handle))
  }

  pub fn end_fn_define(self) -> ShaderUserDefinedFunction {
    modify_graph(|g| g.end_fn_define())
  }
}

// this is useful when define function by derive
pub fn do_return<T>(v: Option<Node<T>>) {
  todo!()
}
// I do this because I don't know how to destruct T from Node<T> in proc macro syc ast, sad!
pub trait ProcMacroNodeHelper {
  type NodeType;
}
impl<T> ProcMacroNodeHelper for Node<T> {
  type NodeType = T;
}

pub fn shader_fn_call(
  meta: ShaderUserDefinedFunction,
  parameters: Vec<ShaderGraphNodeRawHandle>,
) -> ShaderGraphNodeRawHandle {
  modify_graph(|g| {
    let expr = ShaderGraphNodeExpr::FunctionCall {
      meta: ShaderFunctionType::Custom(todo!()),
      parameters,
    };
    g.make_expression(expr)
  })
}
