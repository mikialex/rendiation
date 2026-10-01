use crate::*;

pub(crate) struct FunctionBuilder {
  /// unique in the module, the entry function is 0
  pub(crate) id: u32,
  pub(crate) function: naga::Function,
  /// used to resolve the expression types of the function
  typifier: naga::front::Typifier,
  /// the open blocks, frames[0] is the function body
  pub(crate) frames: Vec<BlockFrame>,
  /// the expressions of the global variables used in this function
  global_exprs: FastHashMap<naga::Handle<naga::GlobalVariable>, naga::Handle<naga::Expression>>,
  /// the expressions of the constants used in this function
  pub(crate) constant_exprs:
    FastHashMap<naga::Handle<naga::Constant>, naga::Handle<naga::Expression>>,
}

pub(crate) struct BlockFrame {
  pub(crate) statements: Vec<naga::Statement>,
  pub(crate) kind: FrameKind,
}

/// The kind of the block frame, it holds the pending control structure which is completed when
/// the frame is popped.
pub(crate) enum FrameKind {
  Body,
  IfAccept {
    condition: naga::Handle<naga::Expression>,
  },
  Else {
    condition: naga::Handle<naga::Expression>,
    accept: naga::Block,
  },
  Loop,
  /// Pushed by begin_switch and popped by end_switch, the popped SwitchCase frame appends the
  /// case here. It does not hold statements, see [FunctionBuilder::statements_mut].
  Switch {
    selector: naga::Handle<naga::Expression>,
    cases: Vec<naga::SwitchCase>,
  },
  SwitchCase(naga::SwitchValue),
}

impl FunctionBuilder {
  pub(crate) fn new(id: u32, function: naga::Function) -> Self {
    let mut builder = Self {
      id,
      function,
      typifier: Default::default(),
      frames: Default::default(),
      global_exprs: Default::default(),
      constant_exprs: Default::default(),
    };
    builder.push_frame(FrameKind::Body);
    builder
  }

  pub(crate) fn push_frame(&mut self, kind: FrameKind) {
    self.frames.push(BlockFrame {
      statements: Default::default(),
      kind,
    });
  }

  pub(crate) fn top_frame_mut(&mut self) -> &mut BlockFrame {
    self.frames.last_mut().unwrap()
  }

  /// The statements of the innermost frame that holds statements. The switch frame is skipped,
  /// the statements pushed between its cases go to the enclosing block, in front of the switch.
  pub(crate) fn statements_mut(&mut self) -> &mut Vec<naga::Statement> {
    let frame = self
      .frames
      .iter_mut()
      .rev()
      .find(|frame| !matches!(frame.kind, FrameKind::Switch { .. }))
      .unwrap();
    &mut frame.statements
  }

  /// Append the expression, and emit it if required.
  fn append_expr(&mut self, expr: naga::Expression) -> naga::Handle<naga::Expression> {
    let needs_pre_emit = expr.needs_pre_emit();
    let handle = self.function.expressions.append(expr, Span::UNDEFINED);

    // should we merge these expression emits?
    if !needs_pre_emit {
      self
        .statements_mut()
        .push(naga::Statement::Emit(naga::Range::new_from_bounds(
          handle, handle,
        )));
    }

    handle
  }

  /// The expression of the global variable in this function, created on first use. It does not
  /// need to be emitted.
  pub(crate) fn global_expr(
    &mut self,
    global: naga::Handle<naga::GlobalVariable>,
  ) -> naga::Handle<naga::Expression> {
    let expressions = &mut self.function.expressions;
    *self.global_exprs.entry(global).or_insert_with(|| {
      expressions.append(naga::Expression::GlobalVariable(global), Span::UNDEFINED)
    })
  }

  /// The expression of the constant in this function, created on first use. It does not need to
  /// be emitted.
  pub(crate) fn constant_expr(
    &mut self,
    constant: naga::Handle<naga::Constant>,
  ) -> naga::Handle<naga::Expression> {
    let expressions = &mut self.function.expressions;
    *self
      .constant_exprs
      .entry(constant)
      .or_insert_with(|| expressions.append(naga::Expression::Constant(constant), Span::UNDEFINED))
  }

  fn resolve_expr_type(
    &mut self,
    module: &naga::Module,
    expr: naga::Handle<naga::Expression>,
  ) -> naga::proc::TypeResolution {
    let ctx = naga::proc::ResolveContext::with_locals(
      module,
      &self.function.local_variables,
      &self.function.arguments,
    );
    self
      .typifier
      .grow(expr, &self.function.expressions, &ctx)
      .expect("failed to resolve the expression type");
    self.typifier[expr].clone()
  }

  /// Close the function body, the other frames must be closed.
  pub(crate) fn finish(mut self) -> naga::Function {
    assert!(
      self.frames.len() == 1,
      "the shader scopes are not balanced when finishing the function, some scope is not closed"
    );
    let body = self.frames.pop().unwrap();
    self.function.body = naga::Block::from_vec(body.statements);
    self.function
  }
}

impl ShaderAPINagaImpl {
  pub(crate) fn building_fn(&self) -> &FunctionBuilder {
    self.functions.last().unwrap()
  }

  pub(crate) fn building_fn_mut(&mut self) -> &mut FunctionBuilder {
    self.functions.last_mut().unwrap()
  }

  pub(crate) fn push_top_statement(&mut self, st: naga::Statement) {
    self.building_fn_mut().statements_mut().push(st);
  }

  /// Append the expression into the building function, and emit it if required.
  pub(crate) fn append_fn_expr(
    &mut self,
    expr: naga::Expression,
  ) -> naga::Handle<naga::Expression> {
    self.building_fn_mut().append_expr(expr)
  }

  pub(crate) fn make_expression_inner(&mut self, expr: naga::Expression) -> ShaderNodeRawHandle {
    let expr = self.append_fn_expr(expr);
    self.map_new_node(expr)
  }

  /// Resolve the type of the expression in the building function.
  pub(crate) fn resolve_expr_type(
    &mut self,
    expr: naga::Handle<naga::Expression>,
  ) -> naga::proc::TypeResolution {
    let function = self.functions.last_mut().unwrap();
    function.resolve_expr_type(&self.module, expr)
  }

  pub(crate) fn add_fn_input_inner(
    &mut self,
    input: naga::FunctionArgument,
  ) -> ShaderNodeRawHandle {
    let arguments = &mut self.building_fn_mut().function.arguments;
    let idx = arguments.len() as u32;
    arguments.push(input);
    self.make_expression_inner(naga::Expression::FunctionArgument(idx))
  }
}
