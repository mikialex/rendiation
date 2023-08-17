use crate::*;

pub struct LoopCtx;

pub fn loop_by(f: impl Fn(LoopCtx)) {
  loop_by_ok(|cx| {
    f(cx);
    Ok(())
  })
  .unwrap()
}

pub fn loop_by_ok(
  f: impl Fn(LoopCtx) -> Result<(), ShaderBuildError>,
) -> Result<(), ShaderBuildError> {
  call_shader_api(|g| g.push_loop_scope());
  f(LoopCtx)?;
  call_shader_api(|g| g.pop_scope());
  Ok(())
}

impl LoopCtx {
  pub fn do_continue(&self) {
    call_shader_api(|g| g.do_continue());
  }
  pub fn do_break(&self) {
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
  pub fn do_continue(&self) {
    call_shader_api(|g| g.do_continue());
  }
  pub fn do_break(&self) {
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

  let index = val(0).make_local_var();
  let condition = val(false).make_local_var();

  fn get_item<T: ShaderNodeType>(
    iter: &ShaderIterator,
    index: &LocalVarNode<u32>,
  ) -> ShaderNodeRawHandle {
    match &iter {
      ShaderIterator::Const(_) => index.load().handle(),
      ShaderIterator::Count(_) => index.load().handle(),
      ShaderIterator::FixedArray { array, .. } => {
        let array: Node<[T; 0]> = unsafe { array.into_node() };
        array.index(index.load()).handle()
      }
      ShaderIterator::Clamped { source, .. } => get_item::<T>(source, index),
    }
  }

  loop_by_ok(|cx| {
    let compare = match &iter {
      ShaderIterator::Const(count) => index.load().less_than(val(*count)),
      ShaderIterator::Count(count) => index.load().less_than(unsafe { count.into_node() }),
      ShaderIterator::FixedArray { length, .. } => index.load().less_than(val(*length as u32)),
      ShaderIterator::Clamped { max, .. } => index.load().less_than(unsafe { max.into_node() }),
    };
    condition.store(compare);
    if_by(condition.load(), || cx.do_break());

    logic(
      &ForCtx,
      unsafe { get_item::<T::Item>(&iter, &index).into_node() },
      index.load(),
    )?;

    index.store(index.load() + val(1));

    Ok(())
  })
}

pub struct ElseEmitter;

impl ElseEmitter {
  pub fn by_else_ok(
    self,
    logic: impl Fn() -> Result<(), ShaderBuildError>,
  ) -> Result<(), ShaderBuildError> {
    call_shader_api(|builder| {
      builder.push_else_scope();
    });

    logic()?;

    call_shader_api(|g| g.pop_scope());
    Ok(())
  }

  pub fn else_by(self, logic: impl Fn()) {
    self
      .by_else_ok(|| {
        logic();
        Ok(())
      })
      .unwrap()
  }
}

#[inline(never)]
pub fn if_by(condition: impl Into<Node<bool>>, logic: impl Fn()) -> ElseEmitter {
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
) -> Result<ElseEmitter, ShaderBuildError> {
  let condition = condition.into().handle();
  call_shader_api(|builder| {
    builder.push_if_scope(condition);
  });

  logic()?;

  call_shader_api(|g| g.pop_scope());

  Ok(ElseEmitter)
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

  pub fn end_with_default(self, default: impl FnOnce()) {
    call_shader_api(|g| g.push_switch_case_scope(SwitchCaseCondition::Default));
    default();
    call_shader_api(|g| {
      g.pop_scope();
      g.end_switch();
    });
  }
}

pub fn switch_by<T>(selector: Node<T>) -> SwitchBuilder<T> {
  call_shader_api(|g| g.begin_switch(selector.handle()));
  SwitchBuilder(Default::default())
}
