use std::{
  cell::Cell,
  sync::{Arc, Mutex},
};

use crate::*;

pub struct LinkListPool<T> {
  pool: IndexReusedVec<LinkListNode<T>>,
}

impl<T> Default for LinkListPool<T> {
  fn default() -> Self {
    Self {
      pool: Default::default(),
    }
  }
}

impl<T> LinkListPool<T> {
  pub fn make_list(&mut self) -> ListHandle {
    ListHandle {
      head: u32::MAX,
      tail: u32::MAX,
    }
  }
  pub fn insert(&mut self, list: &mut ListHandle, data: T) -> u32 {
    let idx = self.pool.insert(LinkListNode {
      next: IndexPtr::new(None),
      data,
    });
    if list.head == u32::MAX {
      // list is empty
      list.head = idx;
      list.tail = idx;
    } else {
      let pre = self.pool.get_mut(list.tail);
      pre.next.set(Some(idx as usize));
    }
    idx
  }

  pub fn remove(&mut self, _list: &mut ListHandle, _to_remove_idx: u32) {
    todo!()
  }

  pub fn drop_list(&mut self, mut list: ListHandle) {
    self.visit_and_remove(&mut list, |_| true)
  }

  /// visitor return if should remove
  pub fn visit_and_remove(
    &mut self,
    list: &mut ListHandle,
    mut visitor: impl FnMut(&mut T) -> bool,
  ) {
    let mut previous = None;
    let mut next_to_visit = IndexPtr::new((list.head == u32::MAX).then_some(list.head as usize));
    while let Some(to_visit) = next_to_visit.get() {
      let to_visit = to_visit as u32;
      let data = self.pool.get_mut(to_visit);
      next_to_visit = data.next;
      if visitor(&mut data.data) {
        // remove current node
        if let Some(previous) = previous {
          // if not first, we update previous node's next
          self.pool.get_mut(previous).next = data.next;
        };
        self.pool.remove(to_visit);
      } else {
        // update previous only if we not remove current
        previous = to_visit.into();
      }
    }
  }
}

#[derive(Copy, Clone)]
pub struct ListHandle {
  head: u32,
  tail: u32,
}

struct LinkListNode<T> {
  next: IndexPtr,
  data: T,
}

pub struct LinkListPoolShared<T> {
  pool: Arc<Mutex<LinkListPool<T>>>,
}

pub struct ListInstance<T> {
  pool: LinkListPoolShared<T>,
  handle: Cell<ListHandle>,
}

impl<T> ListInstance<T> {
  pub fn insert(&self, item: T) {
    let mut pool = self.pool.pool.lock().unwrap();
    let mut handle = self.handle.get();
    pool.insert(&mut handle, item);
    self.handle.set(handle);
  }

  /// visitor return if should remove
  pub fn visit_and_remove(&self, f: impl FnMut(&mut T) -> bool) {
    let mut pool = self.pool.pool.lock().unwrap();
    let mut handle = self.handle.get();
    pool.visit_and_remove(&mut handle, f);
    self.handle.set(handle);
  }
}

impl<T> Drop for ListInstance<T> {
  fn drop(&mut self) {
    let mut pool = self.pool.pool.lock().unwrap();
    pool.drop_list(self.handle.get())
  }
}
