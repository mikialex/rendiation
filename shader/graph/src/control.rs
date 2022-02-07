use std::{any::TypeId, collections::HashMap, marker::PhantomData, sync::Mutex};

use crate::*;

#[derive(Clone, Copy)]
pub struct Mutable<T> {
  phantom: PhantomData<T>,
}

impl<T: ShaderGraphNodeType> ShaderGraphNodeType for Mutable<T> {
  fn to_type() -> ShaderValueType {
    T::to_type()
  }
}

impl<T: ShaderGraphNodeType> Node<T> {
  pub fn mutable(&self) -> Node<Mutable<T>> {
    unsafe { self.handle().cast_type().into() }
  }
}

impl<T: ShaderGraphNodeType> Node<Mutable<T>> {
  pub fn get(&self) -> Node<T> {
    unsafe { self.handle().cast_type().into() }
  }

  pub fn set(&self, node: impl Into<Node<T>>) {
    let node = node.into();
    modify_graph(|builder| {
      ShaderGraphNodeData::Write {
        source: node.cast_untyped(),
        target: self.get().cast_untyped(),
      }
      .insert_into_graph::<AnyType>(builder)
    });

    // unsafe { self.handle.set(handle.cast_type()) };
  }
}

#[must_use]
pub fn consts<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  v.into()
}

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
    let id = builder.push_scope().graph_guid;

    (ShaderGraphNodeData::UnNamed.insert_into_graph(builder), id)
  });
  let cx = ForCtx { target_scope_id };

  logic(&cx, i_node);

  modify_graph(|builder| {
    let scope = builder.pop_scope();

    ShaderControlFlowNode::For {
      source: iterable.into(),
      scope,
    }
    .insert_into_graph(builder)
  });
}

pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) {
  modify_graph(|builder| {
    builder.push_scope();
  });

  logic();

  modify_graph(|builder| {
    let scope = builder.pop_scope();
    let condition = condition.into().cast_untyped();

    ShaderControlFlowNode::If { condition, scope }.insert_into_graph(builder);
  });
}

pub struct FragmentCtx;

impl FragmentCtx {
  pub fn discard() {
    ShaderSideEffectNode::Termination.insert_graph_bottom();
  }
}

/// you can only return the current function, so we don't need
/// FunctionCtx to hold this function
pub fn early_return<T>(return_value: impl Into<Node<T>>) {
  ShaderSideEffectNode::Return(return_value.into().cast_untyped()).insert_graph_bottom();
}

/// use runtime leak to statically store the user gen function
pub static GLOBAL_USER_FUNCTIONS: once_cell::sync::Lazy<
  Mutex<HashMap<TypeId, &'static ShaderFunctionMetaInfo>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(Default::default()));

pub trait IntoParam {
  fn into_param(self) -> Vec<ShaderGraphNodeRawHandleUntyped>;
}

impl<A, B> IntoParam for (A, B) {
  fn into_param(self) -> Vec<ShaderGraphNodeRawHandleUntyped> {
    todo!()
  }
}

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
