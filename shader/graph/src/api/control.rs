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

impl From<u32> for ShaderIterator {
  fn from(v: u32) -> Self {
    ShaderIterator::Const(v)
  }
}

impl ShaderIteratorAble for u32 {
  type Item = u32;
}

impl From<Node<u32>> for ShaderIterator {
  fn from(v: Node<u32>) -> Self {
    ShaderIterator::Count(v.handle())
  }
}

impl ShaderIteratorAble for Node<u32> {
  type Item = u32;
}

impl<T, const U: usize> From<Node<Shader140Array<T, U>>> for ShaderIterator {
  fn from(v: Node<Shader140Array<T, U>>) -> Self {
    ShaderIterator::FixedArray {
      array: v.handle(),
      length: U,
    }
  }
}

impl<T: ShaderGraphNodeType, const U: usize> ShaderIteratorAble for Node<Shader140Array<T, U>> {
  type Item = T;
}

impl<T: ShaderGraphNodeType, const U: usize> ShaderIteratorAble for Node<[T; U]> {
  type Item = T;
}

impl<T: ShaderGraphNodeType, const U: usize> ShaderIteratorAble for Node<BindingArray<T, U>> {
  type Item = T;
}

pub struct ClampedShaderIter<T> {
  pub source: T,
  pub count: Node<u32>,
}

impl<T: Into<ShaderIterator>> From<ClampedShaderIter<T>> for ShaderIterator {
  fn from(v: ClampedShaderIter<T>) -> Self {
    ShaderIterator::Clamped {
      source: Box::new(v.source.into()),
      max: v.count.handle(),
    }
  }
}

impl<T: ShaderIteratorAble> ShaderIteratorAble for ClampedShaderIter<T> {
  type Item = T::Item;
}

#[inline(never)]
pub fn for_by<T: Into<ShaderIterator> + ShaderIteratorAble>(
  iterable: T,
  logic: impl Fn(&ForCtx, Node<T::Item>, Node<u32>),
) where
  T::Item: ShaderGraphNodeType,
{
  for_by_ok(iterable, |ctx, i, v| {
    logic(ctx, i, v);
    Ok(())
  })
  .unwrap()
}

#[inline(never)]
pub fn for_by_ok<T: Into<ShaderIterator> + ShaderIteratorAble>(
  iterable: T,
  logic: impl Fn(&ForCtx, Node<T::Item>, Node<u32>) -> Result<(), ShaderGraphBuildError>,
) -> Result<(), ShaderGraphBuildError>
where
  T::Item: ShaderGraphNodeType,
{
  let (item_node, index_node, target_scope_id) = modify_graph(|builder| {
    let item_node = ShaderGraphNode::UnNamed.insert_into_graph(builder);
    let index_node = ShaderGraphNode::UnNamed.insert_into_graph::<u32>(builder);
    let id = builder.push_scope().graph_guid;

    (item_node, index_node, id)
  });
  let cx = ForCtx { target_scope_id };

  logic(&cx, item_node, index_node)?;

  modify_graph(|builder| {
    let scope = builder.pop_scope();

    ShaderControlFlowNode::For {
      source: iterable.into(),
      scope,
      iter: item_node.handle(),
      index: index_node.handle(),
    }
    .insert_into_graph(builder)
  });

  Ok(())
}

#[inline(never)]
pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) {
  if_by_ok(condition, || {
    logic();
    Ok(())
  })
  .unwrap()
}

#[inline(never)]
pub fn if_by_ok(
  condition: impl Into<Node<bool>>,
  logic: impl Fn() -> Result<(), ShaderGraphBuildError>,
) -> Result<(), ShaderGraphBuildError> {
  let condition = condition.into();
  modify_graph(|builder| {
    builder.push_scope();
  });

  logic()?;

  modify_graph(|builder| {
    let scope = builder.pop_scope();
    let condition = condition.handle();

    ShaderControlFlowNode::If { condition, scope }.insert_into_graph(builder);
  });

  Ok(())
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

//   ShaderGraphNode::Function(FunctionNode {
//     prototype: meta,
//     parameters: todo!(),
//   })
//   .insert_graph()
// }
