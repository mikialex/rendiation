use crate::*;

pub enum ShaderFunctionType {
  Custom(&'static ShaderFunctionMetaInfo),
  BuiltIn(ShaderBuiltInFunction),
}

/// use for compile time ubo field reflection by procedure macro;
#[derive(Debug, Clone)]
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

pub struct FunctionBuildCtx<T>(PhantomData<T>);

pub enum ShaderFnTryDefineResult<T> {
  NotDefined(FunctionBuildCtx<T>),
  AlreadyDefined(ShaderFunctionMetaInfo),
}

impl<T: ShaderGraphNodeType> ShaderFnTryDefineResult<T> {
  pub fn or_define(self, f: impl FnOnce(&FunctionBuildCtx<T>)) -> ShaderFunctionMetaInfo {
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

  pub fn end_fn_define(self) -> ShaderFunctionMetaInfo {
    modify_graph(|g| g.end_fn_define())
  }
}

// I do this because I don't know how to destruct T from Node<T> in proc macro syc ast, sad!
pub trait ProcMacroNodeHelper {
  type NodeType;
}
impl<T> ProcMacroNodeHelper for Node<T> {
  type NodeType = T;
}

pub fn shader_fn_call(
  meta: ShaderFunctionMetaInfo,
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
