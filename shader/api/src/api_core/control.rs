use crate::*;

pub struct LoopCtx;

pub fn loop_by(f: impl FnOnce(LoopCtx)) {
  call_shader_api(|g| g.push_loop_scope());
  f(LoopCtx);
  call_shader_api(|g| g.pop_scope());
}

impl LoopCtx {
  pub fn do_continue(&self) {
    call_shader_api(|g| g.do_continue());
  }
  pub fn do_break(&self) {
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

pub fn return_value<T>(v: Option<Node<T>>) {
  call_shader_api(|g| g.do_return(v.map(|v| v.handle())))
}

pub fn do_return() {
  call_shader_api(|g| g.do_return(None))
}
