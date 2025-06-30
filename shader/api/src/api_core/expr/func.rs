use crate::*;

pub enum ShaderFunctionType {
  Custom(ShaderUserDefinedFunction),
  BuiltIn {
    ty: ShaderBuiltInFunction,
    /// this is a workaround for avoid introducing type infer for our current naga backend
    ty_help_info: Option<PrimitiveShaderValueType>,
  },
}

#[derive(Clone)]
pub struct ShaderUserDefinedFunction {
  pub name: String,
}

#[derive(Clone)]
pub struct ShaderUserDefinedFunctionTyped<T> {
  pub inner: ShaderUserDefinedFunction,
  p: PhantomData<T>,
}

pub struct ShaderUserDefinedFunctionCaller<T> {
  inner: ShaderUserDefinedFunction,
  params: Vec<ShaderNodeRawHandle>,
  p: PhantomData<T>,
}

impl<T> ShaderUserDefinedFunctionCaller<T> {
  // todo, make it type safe
  pub fn push<X>(mut self, p: Node<X>) -> Self {
    self.params.push(p.handle());
    self
  }

  pub fn call(self) -> Node<T> {
    unsafe { shader_fn_call(self.inner, self.params).into_node() }
  }
}

impl<T> ShaderUserDefinedFunctionTyped<T> {
  pub fn prepare_parameters(self) -> ShaderUserDefinedFunctionCaller<T> {
    ShaderUserDefinedFunctionCaller {
      inner: self.inner,
      params: Default::default(),
      p: PhantomData,
    }
  }
}

pub struct FunctionBuildCtx<T>(PhantomData<T>);

pub enum ShaderFnTryDefineResult<T> {
  NotDefined(FunctionBuildCtx<T>),
  AlreadyDefined(ShaderUserDefinedFunction),
}

impl<T: ShaderNodeType> ShaderFnTryDefineResult<T> {
  pub fn or_define(
    self,
    f: impl FnOnce(&FunctionBuildCtx<T>),
  ) -> ShaderUserDefinedFunctionTyped<T> {
    let inner = match self {
      ShaderFnTryDefineResult::NotDefined(builder) => {
        f(&builder);
        builder.end_fn_define()
      }
      ShaderFnTryDefineResult::AlreadyDefined(meta) => meta,
    };
    ShaderUserDefinedFunctionTyped {
      inner,
      p: PhantomData,
    }
  }
}

pub fn shader_fn_name<T>(f: T) -> String {
  std::any::type_name_of_val(&f).to_owned()
}

// todo check T match returned meta
// todo, shader fn macro should check code not use rust return!
pub fn get_shader_fn<T: ShaderNodeType>(name: String) -> ShaderFnTryDefineResult<T> {
  let info = call_shader_api(|g| g.get_fn(name.clone()));

  match info {
    Some(info) => ShaderFnTryDefineResult::AlreadyDefined(info),
    None => ShaderFnTryDefineResult::NotDefined(FunctionBuildCtx::begin(name)),
  }
}

impl<T: ShaderNodeType> FunctionBuildCtx<T> {
  pub fn begin(name: String) -> Self {
    let ty = T::ty();
    let ty = match ty {
      ShaderValueType::Never => None,
      _ => Some(ty),
    };
    call_shader_api(|g| g.begin_define_fn(name, ty));
    Self(Default::default())
  }

  pub fn push_fn_parameter<P: ShaderNodeType>(&self) -> Node<P> {
    unsafe { call_shader_api(|g| g.push_fn_parameter(P::ty())).into_node() }
  }
  pub fn push_fn_parameter_by<P: ShaderNodeType>(&self, _node: Node<P>) -> Node<P> {
    unsafe { call_shader_api(|g| g.push_fn_parameter(P::ty())).into_node() }
  }

  pub fn do_return(&self, r: impl Into<Node<T>>) {
    let handle = match T::ty() {
      ShaderValueType::Never => None,
      _ => Some(r.into().handle()),
    };
    call_shader_api(|g| g.do_return(handle))
  }

  pub fn end_fn_define(self) -> ShaderUserDefinedFunction {
    call_shader_api(|g| g.end_fn_define())
  }
}

// This util trait makes easy to extract T from Node<T> in proc macro syc ast.
pub trait ProcMacroNodeHelper {
  type NodeType;
}
impl<T> ProcMacroNodeHelper for Node<T> {
  type NodeType = T;
}

pub fn shader_fn_call(
  meta: ShaderUserDefinedFunction,
  parameters: Vec<ShaderNodeRawHandle>,
) -> ShaderNodeRawHandle {
  call_shader_api(|g| {
    let expr = ShaderNodeExpr::FunctionCall {
      meta: ShaderFunctionType::Custom(meta),
      parameters,
    };
    g.make_expression(expr)
  })
}
