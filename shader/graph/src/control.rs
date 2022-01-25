use std::{hash::Hasher, marker::PhantomData};

use crate::*;

pub struct NodeScope {
  environment: Vec<NodeUntyped>,
}

pub struct IfNode {
  pub condition: Node<bool>,
  // should be same type
  pub true_value: ShaderGraphNodeRawHandleUntyped,
  pub false_value: ShaderGraphNodeRawHandleUntyped,
}

// impl ShaderIterator for Node<ShaderArray<T>> {
//   type Item = T;

//   fn code_gen(&self) -> &'static str {
//     "
//         for(int i = 0; i < 32; i++) {

//         }
//         "
//   }
// }

// let a = 1;
// let c = 0;
// for i in xxx {
//     let b =1;
//     if i> 10 {
//         a+=b
//         continue
//     }
//     c+= i;
// }

// fn test() {
//   let a = node(1);
//   let c = node(0);
//   let b = node(1);
//   xxx.iter().split(
//     until(10).fold(a, |a| a + b),
//     enumerate().fold(a, |a, i| a + i),
//   );
// }

fn test() {
  let a = consts(1).mutable();
  let c = consts(0).mutable();

  for_by(5, |for_ctx, i| {
    let b = 1;
    if_by(i.greater_than(0), || {
      a.set(a.get() + b.into());
      for_ctx.do_continue();
    });
    c.set(c.get() + i);
  });
}

pub struct Mutable<T> {
  phantom: PhantomData<T>,
}

impl<T> Node<T> {
  pub fn mutable(&self) -> Node<Mutable<T>> {
    todo!()
  }
}

impl<T> Node<Mutable<T>> {
  pub fn get(&self) -> Node<T> {
    todo!()
  }

  pub fn set(&self, node: Node<T>) {
    //
  }
}

pub struct ShaderArray<T> {
  phantom: PhantomData<T>,
}

pub trait ShaderIterator {
  type Item;

  fn code_gen(&self, iter_item_name: &str) -> String;
}

pub fn consts<T>(v: T) -> Node<T> {
  todo!()
}

pub struct ForCtx;

impl ForCtx {
  pub fn do_continue(&self) {
    modify_graph(|builder| {
      let scope = builder.top_scope();
      // todo insert node?
      scope.code_builder.write_ln("continue");
    });
  }

  pub fn do_break(&self) {
    modify_graph(|builder| {
      let scope = builder.top_scope();
      // todo insert node?
      scope.code_builder.write_ln("break");
    });
  }
}

impl ShaderIterator for u32 {
  type Item = u32;

  fn code_gen(&self, iter_item_name: &str) -> String {
    format!(
      "for (int {name} = 0; {name} < {count}; ++i)",
      name = iter_item_name,
      count = self
    )
  }
}

impl<T> ShaderIterator for ShaderArray<T> {
  type Item = T;

  fn code_gen(&self, iter_item_name: &str) -> String {
    todo!()
    // format!(
    //   "for (int {name} = 0; {name} < {count}; ++i)",
    //   name = iter_item_name,
    //   count = self
    // )
  }
}

pub fn for_by<T, I>(iterable: I, logic: impl Fn(&ForCtx, Node<T>))
where
  T: ShaderGraphNodeType,
  I: ShaderIterator<Item = T>,
{
  let i_node = modify_graph(|builder| {
    let scope = builder.top_scope();
    let iter_item_name = scope.code_gen.create_new_unique_name();
    scope
      .code_builder
      .write_ln(iterable.code_gen(iter_item_name.as_ref()).as_str());

    scope.code_builder.tab();
    let for_body = builder.push_scope();

    ShaderGraphNodeData::Named(iter_item_name.into()).insert_into_graph(for_body)
  });

  let cx = ForCtx;

  logic(&cx, i_node);

  modify_graph(|builder| {
    let result = builder.pop_scope();
    let result = ShaderGraphNodeData::Scope(result);

    let scope = builder.top_scope();
    result.insert_into_graph::<AnyType>(scope);

    scope.code_builder.un_tab();
    scope.code_builder.write_ln("}");
  });
}

pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) {
  modify_graph(|builder| {
    let condition = builder.get_node_gen_result_var(condition);
    let scope = builder.top_scope();
    scope
      .code_builder
      .write_ln(format!("if ({}) {{", condition).as_str());

    scope.code_builder.tab();
    builder.push_scope();
  });

  logic();

  modify_graph(|builder| {
    let result = builder.pop_scope();
    let result = ShaderGraphNodeData::Scope(result);

    let scope = builder.top_scope();
    result.insert_into_graph::<AnyType>(scope);

    scope.code_builder.un_tab();
    scope.code_builder.write_ln("}");
  });
}

pub struct FragmentCtx;

impl FragmentCtx {
  pub fn discard() {
    modify_graph(|builder| {
      let scope = builder.top_scope();
      scope.code_builder.write_ln("discard;");
    });
  }
}

pub struct FunctionCtx<T> {
  phantom: PhantomData<T>,
}

impl<T> FunctionCtx<T> {
  // how do we validate the ast generated match the function definition?
  pub fn do_return(&self, return_value: Node<T>) {
    //
  }
}
