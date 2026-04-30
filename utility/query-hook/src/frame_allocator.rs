use std::{alloc::Allocator, cell::RefCell, sync::atomic::AtomicUsize};

use bumpalo::Bump;

use crate::*;

static GLOBAL_LIVING_BUMP: AtomicUsize = AtomicUsize::new(0);
pub fn get_global_living_bump() -> usize {
  GLOBAL_LIVING_BUMP.load(std::sync::atomic::Ordering::Relaxed)
}

struct BumpWrap(Bump);

impl Drop for BumpWrap {
  fn drop(&mut self) {
    GLOBAL_LIVING_BUMP.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
  }
}

#[derive(Clone)]
pub struct FrameAlloc {
  bump: Arc<BumpWrap>,
}

impl FrameAlloc {
  pub fn new(bytes_capacity: usize) -> Self {
    GLOBAL_LIVING_BUMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Self {
      bump: Arc::new(BumpWrap(Bump::with_capacity(bytes_capacity))),
    }
  }
}

/// we get frame allocator from thread_local, so it guaranteed all allocation for one FrameAlloc
/// are from one thread
unsafe impl Sync for FrameAlloc {}
/// although the FrameAlloc may be sent to other thread, but nothing can be called in this case
/// even deallocation tail check is skipped
unsafe impl Send for FrameAlloc {}

unsafe impl Allocator for FrameAlloc {
  fn allocate(
    &self,
    layout: std::alloc::Layout,
  ) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
    (&self.bump.as_ref().0).allocate(layout)
  }

  unsafe fn deallocate(&self, _ptr: std::ptr::NonNull<u8>, _layout: std::alloc::Layout) {
    // we not try to deallocate at all, because this case may happen in other thread
  }
}

thread_local! {
  static FRAME_ALLOC: RefCell<Option<FrameAlloc>> = const { RefCell::new(None) };
}

pub fn setup_new_frame_allocator(bytes_capacity: usize) {
  FRAME_ALLOC.with(|f| *f.borrow_mut() = FrameAlloc::new(bytes_capacity).into());
}

pub fn box_in_frame<T>(item: T) -> FrameBox<T> {
  let current = FRAME_ALLOC.with(|f| f.borrow().clone()).unwrap();
  Box::new_in(item, current)
}

pub fn pin_box_in_frame<T>(item: T) -> Pin<FrameBox<T>> {
  unsafe { Pin::new_unchecked(box_in_frame(item)) }
}

pub type FrameBox<T> = Box<T, FrameAlloc>;
