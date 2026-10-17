use std::alloc::{Allocator, Global};
use std::cell::RefCell;

use bumpalo::Bump;

/// A bump allocator reused by short lived allocations. The memory is reset after each scope, so
/// the allocations of repeated scopes reuse the same memory.
#[derive(Default)]
pub struct ScratchAllocator {
  bump: RefCell<Bump>,
}

impl ScratchAllocator {
  /// The result can not borrow the allocator, so everything allocated in the scope is dropped
  /// before the reset. A nested scope falls back to the global allocator, because the outer scope
  /// still holds the memory.
  pub fn scope<R>(&self, f: impl FnOnce(&dyn Allocator) -> R) -> R {
    let Ok(mut bump) = self.bump.try_borrow_mut() else {
      return f(&Global);
    };
    let alloc = &*bump;
    let r = f(&alloc);
    bump.reset();
    r
  }
}
