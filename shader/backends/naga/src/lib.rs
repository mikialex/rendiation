use shadergraph::*;

#[derive(Default)]
pub struct ShaderAPINagaImpl;

impl ShaderAPI for ShaderAPINagaImpl {
  fn register_ty(&mut self, ty: ShaderValueType) {
    todo!()
  }

  fn make_expression(&mut self, expr: ShaderGraphNodeExpr) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn define_input(&mut self, input: ShaderGraphInputNode) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn push_scope(&mut self) {
    todo!()
  }

  fn pop_scope(&mut self) {
    todo!()
  }

  fn push_if_scope(&mut self, condition: ShaderGraphNodeRawHandle) {
    todo!()
  }

  fn discard(&mut self) {
    todo!()
  }

  fn push_for_scope(&mut self, target: ShaderIterator) -> ForNodes {
    todo!()
  }

  fn do_continue(&mut self, looper: ShaderGraphNodeRawHandle) {
    todo!()
  }

  fn do_break(&mut self, looper: ShaderGraphNodeRawHandle) {
    todo!()
  }

  fn make_var(&mut self) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn write(&mut self, source: ShaderGraphNodeRawHandle, target: ShaderGraphNodeRawHandle) {
    todo!()
  }
  fn load(&mut self, source: ShaderGraphNodeRawHandle) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn build(&mut self) -> (String, String) {
    todo!()
  }

  fn define_frag_out(&mut self, idx: usize) -> ShaderGraphNodeRawHandle {
    todo!()
  }
}
