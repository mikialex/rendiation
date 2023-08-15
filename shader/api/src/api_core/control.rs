use crate::*;

pub struct WhileCtx;

pub fn while_by(condition: &NodeMutable<bool>, f: impl Fn(WhileCtx)) {
  while_by_ok(condition, |cx| {
    f(cx);
    Ok(())
  })
  .unwrap()
}

pub fn while_by_ok(
  condition: &NodeMutable<bool>,
  f: impl Fn(WhileCtx) -> Result<(), ShaderBuildError>,
) -> Result<(), ShaderBuildError> {
  call_shader_api(|g| g.push_while_scope(condition.inner.handle()));
  f(WhileCtx)?;
  call_shader_api(|g| g.pop_scope());
  Ok(())
}

impl WhileCtx {
  // note, we here use &mut self, is to prevent usage of nested continue statement.
  pub fn do_continue(&mut self) {
    call_shader_api(|g| g.do_continue());
  }
  // ditto
  pub fn do_break(&mut self) {
    call_shader_api(|g| g.do_break());
  }
}

pub trait ShaderIteratorAble {
  type Item: ShaderNodeType;
}

pub enum ShaderIterator {
  Const(u32),
  Count(ShaderNodeRawHandle),
  FixedArray {
    array: ShaderNodeRawHandle,
    length: usize,
  },
  Clamped {
    source: Box<Self>,
    max: ShaderNodeRawHandle,
  },
}

pub struct ForCtx;

pub struct ForNodes {
  pub item_node: ShaderNodeRawHandle,
  pub index_node: ShaderNodeRawHandle,
  pub for_cx: ShaderNodeRawHandle,
}

impl ForCtx {
  // note, we here use &mut self, is to prevent usage of nested continue statement.
  pub fn do_continue(&mut self) {
    call_shader_api(|g| g.do_continue());
  }
  // ditto
  pub fn do_break(&mut self) {
    call_shader_api(|g| g.do_break());
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

impl<T: ShaderNodeType, const U: usize> ShaderIteratorAble for Node<Shader140Array<T, U>> {
  type Item = T;
}

impl<T: ShaderNodeType, const U: usize> ShaderIteratorAble for Node<[T; U]> {
  type Item = T;
}

impl<T: ShaderNodeType, const U: usize> ShaderIteratorAble for Node<BindingArray<T, U>> {
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
  T::Item: ShaderNodeType,
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
  logic: impl Fn(&ForCtx, Node<T::Item>, Node<u32>) -> Result<(), ShaderBuildError>,
) -> Result<(), ShaderBuildError>
where
  T::Item: ShaderNodeType,
{
  let iter: ShaderIterator = iterable.into();

  let index = val(0).mutable();
  let condition = val(false).mutable();

  let init = match &iter {
    ShaderIterator::Const(count) => index.get().less_than(val(*count)),
    ShaderIterator::Count(_) => todo!(),
    ShaderIterator::FixedArray { length, .. } => index.get().less_than(val(*length as u32)),
    ShaderIterator::Clamped { max, .. } => index.get().less_than(unsafe { max.into_node() }),
  };
  condition.set(init);

  fn get_item<T: ShaderNodeType>(
    iter: &ShaderIterator,
    index: &NodeMutable<u32>,
  ) -> ShaderNodeRawHandle {
    match &iter {
      ShaderIterator::Const(_) => index.get().handle(),
      ShaderIterator::Count(_) => todo!(),
      ShaderIterator::FixedArray { array, .. } => {
        let array: Node<[T; 0]> = unsafe { array.into_node() };
        array.index(index.get()).handle()
      }
      ShaderIterator::Clamped { source, .. } => get_item::<T>(source, index),
    }
  }

  while_by_ok(&condition, |_cx| {
    logic(
      &ForCtx,
      unsafe { get_item::<T::Item>(&iter, &index).into_node() },
      index.get(),
    )?;
    index.set(index.get() + val(1));
    Ok(())
  })
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
  logic: impl Fn() -> Result<(), ShaderBuildError>,
) -> Result<(), ShaderBuildError> {
  let condition = condition.into().handle();
  call_shader_api(|builder| {
    builder.push_if_scope(condition);
  });

  logic()?;

  call_shader_api(|g| g.pop_scope());

  Ok(())
}

pub trait SwitchableShaderType: ShaderNodeType {
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
    call_shader_api(|g| g.push_switch_case_scope(v.into_condition()));
    scope();
    call_shader_api(|g| g.pop_scope());
    self
  }

  pub fn end_with_default(self, scope: impl FnOnce()) {
    call_shader_api(|g| g.push_switch_case_scope(SwitchCaseCondition::Default));
    scope();
    self.end()
  }

  pub fn end(self) {
    call_shader_api(|g| {
      g.pop_scope();
      g.end_switch()
    });
  }
}

pub fn switch_by<T>(selector: Node<T>) -> SwitchBuilder<T> {
  call_shader_api(|g| g.begin_switch(selector.handle()));
  SwitchBuilder(Default::default())
}
