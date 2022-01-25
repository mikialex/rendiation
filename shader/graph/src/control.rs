use std::marker::PhantomData;

use crate::*;

#[derive(Clone, Copy)]
pub struct Mutable<T> {
  phantom: PhantomData<T>,
}

impl<T: ShaderGraphNodeType> ShaderGraphNodeType for Mutable<T> {
  fn to_glsl_type() -> &'static str {
    T::to_glsl_type()
  }
}

impl<T: ShaderGraphNodeType> Node<T> {
  pub fn mutable(&self) -> Node<Mutable<T>> {
    unsafe { self.handle().cast_type().into() }
  }
}

impl<T> Node<Mutable<T>> {
  pub fn get(&self) -> Node<T> {
    todo!()
    // modify_graph(|builder| {
    //   let value = builder.get_node_gen_result_var(self);
    //   let scope = builder.top_scope();
    //   let copied_value = scope.code_gen.create_new_unique_name();
    //   scope
    //     .code_builder
    //     .write_ln(format!("return {};", return_value).as_str());

    // ShaderGraphNodeData::Named(copied_value).insert_into_graph(for_body)
    // });
  }

  pub fn set(&self, node: Node<T>) {
    unsafe { self.handle.set(node.handle().cast_type()) };
    // modify_graph(|builder| {
    //   let value = builder.get_node_gen_result_var(self);
    //   let scope = builder.top_scope();
    //   let copied_value = scope.code_gen.create_new_unique_name();
    //   scope
    //     .code_builder
    //     .write_ln(format!("return {};", return_value).as_str());

    // ShaderGraphNodeData::Named(copied_value).insert_into_graph(for_body)
    // });
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

    ShaderGraphNodeData::Named(iter_item_name).insert_into_graph(for_body)
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
  pub fn do_return(&self, return_value: impl Into<Node<T>>) {
    modify_graph(|builder| {
      let return_value = builder.get_node_gen_result_var(return_value);
      let scope = builder.top_scope();
      scope
        .code_builder
        .write_ln(format!("return {};", return_value).as_str());
    });
  }
}
