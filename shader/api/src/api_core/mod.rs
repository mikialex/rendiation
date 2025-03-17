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

mod into_iter;
pub use into_iter::*;

const ENABLE_SHADER_ASSERTION: bool = true;

/// Assert unreachable execution states reached in shader by triggering an infinite loop.
/// This is useful for debugging because the program may not crash even unreachable case occurred.
/// todo: some gpu venders still may not lost device even infinite loop triggered,
/// we should impl more advance error reporting system to record this kind of diagnostic information.
pub fn shader_unreachable() {
  if !ENABLE_SHADER_ASSERTION {
    return;
  }
  loop_by(|_| {})
}

pub fn shader_assert(cond: Node<bool>) {
  if !ENABLE_SHADER_ASSERTION {
    return;
  }
  if_by(cond.not(), shader_unreachable);
}
