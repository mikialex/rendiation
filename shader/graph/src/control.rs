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
    ShaderGraphNodeData::Copy(self.cast_untyped()).insert_graph()
  }

  pub fn set(&self, node: impl Into<Node<T>>) {
    unsafe { self.handle.set(node.into().handle().cast_type()) };
  }
}

pub trait ShaderIterator {
  type Item;

  fn code_gen(&self, iter_item_name: &str) -> String;
}

#[must_use]
pub fn consts<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  v.into()
}

pub struct ForCtx;

impl ForCtx {
  pub fn do_continue(&self) {
    modify_graph(|builder| {
      // todo insert node?
      builder.code_builder.write_ln("continue");
    });
  }

  pub fn do_break(&self) {
    modify_graph(|builder| {
      // todo insert node?
      builder.code_builder.write_ln("break");
    });
  }
}

impl ShaderIterator for u32 {
  type Item = u32;

  fn code_gen(&self, iter_item_name: &str) -> String {
    format!(
      "for (int {name} = 0; {name} < {count}; ++i) {{",
      name = iter_item_name,
      count = self
    )
  }
}

// pub struct ShaderArray<T> {
//   phantom: PhantomData<T>,
// }

// impl<T> ShaderIterator for ShaderArray<T> {
//   type Item = T;

//   fn code_gen(&self, iter_item_name: &str) -> String {
//     todo!()
//     // format!(
//     //   "for (int {name} = 0; {name} < {count}; ++i)",
//     //   name = iter_item_name,
//     //   count = self
//     // )
//   }
// }

pub fn for_by<T, I>(iterable: I, logic: impl Fn(&ForCtx, Node<T>))
where
  T: ShaderGraphNodeType,
  I: ShaderIterator<Item = T>,
{
  let i_node = modify_graph(|builder| {
    let scope = builder.top_scope();
    let iter_item_name = scope.code_gen.create_new_unique_name();
    builder
      .code_builder
      .write_ln(iterable.code_gen(iter_item_name.as_ref()).as_str());

    builder.push_scope();
    builder.code_builder.tab();

    ShaderGraphNodeData::Named(iter_item_name).insert_into_graph(builder)
  });

  let cx = ForCtx;

  logic(&cx, i_node);

  modify_graph(|builder| {
    builder.code_builder.un_tab();
    builder.code_builder.write_ln("}");

    builder.pop_scope();

    ShaderGraphNodeData::Scope.insert_into_graph::<AnyType>(builder);
  });
}

pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) {
  modify_graph(|builder| {
    let condition = builder.get_node_gen_result_var(condition);
    let condition = format!("if ({}) {{", condition);
    builder.code_builder.write_ln(condition);

    builder.push_scope();
    builder.code_builder.tab();
  });

  logic();

  modify_graph(|builder| {
    builder.code_builder.un_tab();
    builder.code_builder.write_ln("}");
    builder.pop_scope();

    ShaderGraphNodeData::Scope.insert_into_graph::<AnyType>(builder);
  });
}

pub struct FragmentCtx;

impl FragmentCtx {
  pub fn discard() {
    modify_graph(|builder| {
      builder.code_builder.write_ln("discard;");
    });
  }
}

/// you can only return the current function, so we don't need
/// FunctionCtx to hold this function
pub fn early_return<T>(return_value: impl Into<Node<T>>) {
  modify_graph(|builder| {
    let return_value = builder.get_node_gen_result_var(return_value);
    let return_value = format!("return {};", return_value);
    builder.code_builder.write_ln(return_value);
  });
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
