use crate::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlScope {
  Loop(usize),
  Switch,
  /// the loop outside of a function can not be targeted inside the function
  Function,
}

/// The loop, switch and function scopes in building, used to check the target of the break and
/// continue statements.
#[derive(Default)]
pub(crate) struct ControlScopes {
  scopes: Vec<ControlScope>,
  next_loop_id: usize,
}

impl ControlScopes {
  fn push_loop(&mut self) -> usize {
    let id = self.next_loop_id;
    self.next_loop_id += 1;
    self.scopes.push(ControlScope::Loop(id));
    id
  }

  fn pop(&mut self, scope: ControlScope) {
    let popped = self.scopes.pop();
    assert!(popped == Some(scope), "control scope mismatch");
  }

  /// WGSL break exits the nearest loop or switch
  fn check_break_target(&self, loop_id: usize) {
    match self.scopes.last() {
      Some(ControlScope::Loop(id)) if *id == loop_id => {}
      Some(ControlScope::Switch) => panic!(
        "break a loop inside the switch case is not supported, the WGSL break statement only exits the switch"
      ),
      Some(ControlScope::Loop(_)) => panic!(
        "break an outer loop inside the inner loop is not supported, the WGSL break statement only exits the inner loop"
      ),
      _ => panic!("break a loop outside of the loop"),
    }
  }

  /// WGSL continue targets the nearest loop, the switch in between does not matter
  fn check_continue_target(&self, loop_id: usize) {
    let target = self
      .scopes
      .iter()
      .rev()
      .find(|scope| **scope != ControlScope::Switch);
    match target {
      Some(ControlScope::Loop(id)) if *id == loop_id => {}
      Some(ControlScope::Loop(_)) => panic!(
        "continue an outer loop inside the inner loop is not supported, the WGSL continue statement only targets the inner loop"
      ),
      _ => panic!("continue a loop outside of the loop"),
    }
  }
}

pub(crate) fn push_function_control_scope() {
  with_control_scopes(|s| s.scopes.push(ControlScope::Function));
}

pub(crate) fn pop_function_control_scope() {
  with_control_scopes(|s| s.pop(ControlScope::Function));
}

pub struct LoopCtx {
  id: usize,
}

pub fn loop_by(f: impl FnOnce(LoopCtx)) {
  let id = with_control_scopes(|s| s.push_loop());
  call_shader_api(|g| g.push_loop_scope());
  f(LoopCtx { id });
  call_shader_api(|g| g.pop_scope());
  with_control_scopes(|s| s.pop(ControlScope::Loop(id)));
}

impl LoopCtx {
  /// panic if the nearest loop is not this loop, WGSL can not continue an outer loop
  pub fn do_continue(&self) {
    with_control_scopes(|s| s.check_continue_target(self.id));
    call_shader_api(|g| g.do_continue());
  }
  /// panic if the nearest loop or switch is not this loop, WGSL can not break an outer loop, and
  /// the break inside the switch case only exits the switch
  pub fn do_break(&self) {
    with_control_scopes(|s| s.check_break_target(self.id));
    call_shader_api(|g| g.do_break());
  }
}

pub struct ElseEmitter(usize);

impl ElseEmitter {
  pub fn else_if(mut self, condition: impl Into<Node<bool>>, logic: impl FnOnce()) -> ElseEmitter {
    let condition = condition.into().handle();
    call_shader_api(|builder| {
      builder.push_else_scope();
      builder.push_if_scope(condition);
    });
    logic();
    call_shader_api(|api| api.pop_scope());
    self.0 += 1;
    self
  }

  pub fn else_over(self) {
    // closing outer scope
    for _ in 0..self.0 {
      call_shader_api(|g| g.pop_scope());
    }
  }

  pub fn else_by(self, logic: impl FnOnce()) {
    call_shader_api(|builder| {
      builder.push_else_scope();
    });

    logic();

    call_shader_api(|g| g.pop_scope());

    // closing outer scope
    for _ in 0..self.0 {
      call_shader_api(|g| g.pop_scope());
    }
  }
}

pub fn if_by(condition: impl Into<Node<bool>>, logic: impl FnOnce()) -> ElseEmitter {
  let condition = condition.into().handle();
  call_shader_api(|builder| {
    builder.push_if_scope(condition);
  });

  logic();

  call_shader_api(|g| g.pop_scope());

  ElseEmitter(0)
}

impl Node<bool> {
  pub fn select_branched<T: ShaderSizedValueNodeType>(
    self,
    tr: impl FnOnce() -> Node<T>,
    fal: impl FnOnce() -> Node<T>,
  ) -> Node<T> {
    let re = zeroed_val::<T>().make_local_var();
    if_by(self, || {
      re.store(tr());
    })
    .else_by(|| {
      re.store(fal());
    });
    re.load()
  }
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SwitchCaseCondition {
  U32(u32),
  I32(i32),
  Default,
}

pub struct SwitchBuilder<T> {
  phantom: PhantomData<T>,
  ended: bool,
  /// the case selector values must be distinct in WGSL
  cases: Vec<SwitchCaseCondition>,
}

impl<T> Drop for SwitchBuilder<T> {
  fn drop(&mut self) {
    // avoid the double panic abort when the builder is dropped by unwinding
    if !self.ended && !std::thread::panicking() {
      panic!("SwitchBuilder dropped without end_with_default")
    }
  }
}

impl<T: SwitchableShaderType> SwitchBuilder<T> {
  pub fn case(mut self, v: T, scope: impl FnOnce()) -> Self {
    let condition = v.into_condition();
    assert!(
      !self.cases.contains(&condition),
      "switch case selector values must be distinct"
    );
    self.cases.push(condition);
    call_shader_api(|g| g.push_switch_case_scope(condition));
    scope();
    call_shader_api(|g| g.pop_scope());
    self
  }

  pub fn end_with_default(mut self, default: impl FnOnce()) {
    call_shader_api(|g| g.push_switch_case_scope(SwitchCaseCondition::Default));
    default();
    call_shader_api(|g| {
      g.pop_scope();
      g.end_switch();
    });
    with_control_scopes(|s| s.pop(ControlScope::Switch));
    self.ended = true;
  }
}

#[must_use]
pub fn switch_by<T: SwitchableShaderType>(selector: Node<T>) -> SwitchBuilder<T> {
  call_shader_api(|g| g.begin_switch(selector.handle()));
  with_control_scopes(|s| s.scopes.push(ControlScope::Switch));
  SwitchBuilder {
    phantom: PhantomData,
    ended: false,
    cases: Vec::new(),
  }
}

pub fn return_value<T>(v: Option<Node<T>>) {
  call_shader_api(|g| g.do_return(v.map(|v| v.handle())))
}

pub fn do_return() {
  call_shader_api(|g| g.do_return(None))
}
