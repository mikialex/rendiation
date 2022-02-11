use crate::*;

pub struct ForCtx {
  target_scope_id: usize,
}

impl ForCtx {
  pub fn do_continue(&self) {
    ShaderSideEffectNode::Continue.insert_graph(self.target_scope_id);
  }

  pub fn do_break(&self) {
    ShaderSideEffectNode::Break.insert_graph(self.target_scope_id);
  }
}

impl From<u32> for ShaderIteratorAble {
  fn from(v: u32) -> Self {
    ShaderIteratorAble::Const(v)
  }
}

pub fn for_by<T>(iterable: impl Into<ShaderIteratorAble>, logic: impl Fn(&ForCtx, Node<T>))
where
  T: ShaderGraphNodeType,
{
  let (i_node, target_scope_id) = modify_graph(|builder| {
    let node = ShaderGraphNodeData::UnNamed.insert_into_graph(builder);
    let id = builder.push_scope().graph_guid;

    (node, id)
  });
  let cx = ForCtx { target_scope_id };

  logic(&cx, i_node);

  modify_graph(|builder| {
    let scope = builder.pop_scope();

    ShaderControlFlowNode::For {
      source: iterable.into(),
      scope,
      iter: i_node.handle(),
    }
    .insert_into_graph(builder)
  });
}

pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) {
  let condition = condition.into();
  modify_graph(|builder| {
    builder.push_scope();
  });

  logic();

  modify_graph(|builder| {
    let scope = builder.pop_scope();
    let condition = condition.handle();

    ShaderControlFlowNode::If { condition, scope }.insert_into_graph(builder);
  });
}

pub struct FragmentCtx;

impl FragmentCtx {
  pub fn discard() {
    ShaderSideEffectNode::Termination.insert_graph_bottom();
  }
}

// /// you can only return the current function, so we don't need
// /// FunctionCtx to hold this function
// pub fn early_return<T>(return_value: impl Into<Node<T>>) {
//   ShaderSideEffectNode::Return(return_value.into().handle()).insert_graph_bottom();
// }

// /// use runtime leak to statically store the user gen function
// pub static GLOBAL_USER_FUNCTIONS: once_cell::sync::Lazy<
//   Mutex<HashMap<TypeId, &'static ShaderFunctionMetaInfo>>,
// > = once_cell::sync::Lazy::new(|| Mutex::new(Default::default()));

// pub trait IntoParam {
//   fn into_param(self) -> Vec<ShaderGraphNodeRawHandle>;
// }

// impl<A, B> IntoParam for (A, B) {
//   fn into_param(self) -> Vec<ShaderGraphNodeRawHandle> {
//     todo!()
//   }
// }

// pub fn function<T, P>(parameters: P, logic: impl Fn(P) -> Node<T> + Any) -> Node<T>
// where
//   T: ShaderGraphNodeType,
//   P: IntoParam,
// {
//   let mut guard = GLOBAL_USER_FUNCTIONS.lock().unwrap();

//   let meta = guard.entry(logic.type_id()).or_insert_with(|| {
//     todo!();
//   });

//   ShaderGraphNodeData::Function(FunctionNode {
//     prototype: meta,
//     parameters: todo!(),
//   })
//   .insert_graph()
// }
