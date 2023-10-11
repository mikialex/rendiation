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

  pub fn remove(&mut self, list: &mut ListHandle, to_remove_idx: u32) {
    self.visit_and_remove(list, |_, idx| (idx == to_remove_idx, idx != to_remove_idx))
  }

  pub fn drop_list(&mut self, mut list: ListHandle) {
    self.visit_and_remove(&mut list, |_, _| (true, true))
  }

  /// visitor (data, index) return (should remove, should continue)
  pub fn visit_and_remove(
    &mut self,
    list: &mut ListHandle,
    mut visitor: impl FnMut(&mut T, u32) -> (bool, bool),
  ) {
    let mut previous = None;
    let mut next_to_visit = IndexPtr::new((list.head == u32::MAX).then_some(list.head as usize));
    while let Some(to_visit) = next_to_visit.get() {
      let to_visit = to_visit as u32;
      let data = self.pool.get_mut(to_visit);
      next_to_visit = data.next;
      let (should_remove, should_continue) = visitor(&mut data.data, to_visit);
      if should_remove {
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
      if !should_continue {
        return;
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
