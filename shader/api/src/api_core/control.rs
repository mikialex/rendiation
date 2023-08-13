use crate::*;

pub trait ShaderIteratorAble {
  type Item: ShaderGraphNodeType;
}

pub enum ShaderIterator {
  Const(u32),
  Count(ShaderGraphNodeRawHandle),
  FixedArray {
    array: ShaderGraphNodeRawHandle,
    length: usize,
  },
  Clamped {
    source: Box<Self>,
    max: ShaderGraphNodeRawHandle,
  },
}

pub struct ForCtx {
  target_scope_id: ShaderGraphNodeRawHandle,
}

impl ForCtx {
  // note, we here use &mut self, is to prevent usage of nested continue statement.
  // theoretically, we could rewrite control flow to support this feature in the future
  pub fn do_continue(&mut self) {
    modify_graph(|g| g.do_continue(self.target_scope_id));
  }
  // ditto
  pub fn do_break(&mut self) {
    modify_graph(|g| g.do_break(self.target_scope_id));
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
  let ForNodes {
    item_node,
    index_node,
    for_cx,
  } = modify_graph(|g| g.push_for_scope(iterable.into()));

  let cx = ForCtx {
    target_scope_id: for_cx,
  };
  let index_node = unsafe { index_node.into_node() };
  let item_node = unsafe { item_node.into_node() };
  logic(&cx, item_node, index_node)?;

  modify_graph(|g| g.pop_scope());

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
  let condition = condition.into().handle();
  modify_graph(|builder| {
    builder.push_if_scope(condition);
  });

  logic()?;

  modify_graph(|g| g.pop_scope());

  Ok(())
}

pub trait SwitchableShaderType: ShaderGraphNodeType {
  fn into_condition(self) -> SwitchCaseCondition;
}
impl SwitchableShaderType for u32 {
  fn into_condition(self) -> SwitchCaseCondition {
    SwitchCaseCondition::U32(self)
  }
}
impl SwitchableShaderType for i32 {
  fn into_condition(self) -> SwitchCaseCondition {
    SwitchCaseCondition::I32(self)
  }
}

pub enum SwitchCaseCondition {
  U32(u32),
  I32(i32),
  Default,
}

pub struct SwitchBuilder<T>(PhantomData<T>);

impl<T: SwitchableShaderType> SwitchBuilder<T> {
  /// None is the default case
  pub fn case(self, v: T, scope: impl FnOnce()) -> Self {
    modify_graph(|g| g.push_switch_case_scope(v.into_condition()));
    scope();
    modify_graph(|g| g.pop_scope());
    self
  }

  pub fn end_with_default(self, scope: impl FnOnce()) {
    modify_graph(|g| g.push_switch_case_scope(SwitchCaseCondition::Default));
    scope();
    self.end()
  }

  pub fn end(self) {
    modify_graph(|g| {
      g.pop_scope();
      g.end_switch()
    });
  }
}

pub fn switch_by<T>(selector: Node<T>) -> SwitchBuilder<T> {
  modify_graph(|g| g.begin_switch(selector.handle()));
  SwitchBuilder(Default::default())
}
