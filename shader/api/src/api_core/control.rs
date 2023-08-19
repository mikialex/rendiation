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
