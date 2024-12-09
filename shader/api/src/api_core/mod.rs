mod node;
pub use node::*;

mod ty;
pub use ty::*;

mod io;
pub use io::*;

mod expr;
pub use expr::*;

mod control;
pub use control::*;

mod iter;
pub use iter::*;

const ENABLE_SHADER_ASSERTION: bool = true;

pub fn shader_unreachable() {
  if !ENABLE_SHADER_ASSERTION {
    return;
  }
  // assert unreachable in shader by trigger a infinite loop
  // this is useful for debugging because the program may not crash even unreachable case occurred.
  loop_by(|_| {})
}

pub fn shader_assert(cond: Node<bool>) {
  if !ENABLE_SHADER_ASSERTION {
    return;
  }
  if_by(cond.not(), shader_unreachable);
}
