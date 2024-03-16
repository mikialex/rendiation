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
  pub fn shrink_to_fit(&mut self) {
    self.pool.shrink_to_fit()
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
      list.tail = idx;
    }
    idx
  }

  pub fn list_len(&mut self, list: &mut ListHandle) -> usize {
    let mut count = 0;
    self.visit_and_remove(list, |_, _| {
      count += 1;
      (false, true)
    });
    count
  }

  pub fn remove(&mut self, list: &mut ListHandle, to_remove_idx: u32) {
    self.visit_and_remove(list, |_, idx| (idx == to_remove_idx, idx != to_remove_idx))
  }

  pub fn drop_list(&mut self, list: &mut ListHandle) {
    self.visit_and_remove(list, |_, _| (true, true))
  }

  /// visitor (data, index) return (should remove, should continue)
  pub fn visit_and_remove(
    &mut self,
    list: &mut ListHandle,
    mut visitor: impl FnMut(&mut T, u32) -> (bool, bool),
  ) {
    let mut previous = None;
    let mut next_to_visit = IndexPtr::new((list.head != u32::MAX).then_some(list.head as usize));
    while let Some(to_visit) = next_to_visit.get() {
      let to_visit = to_visit as u32;
      let data = self.pool.get_mut(to_visit);
      next_to_visit = data.next;
      let (should_remove, should_continue) = visitor(&mut data.data, to_visit);
      if should_remove {
        if list.head == to_visit {
          list.head = data.next.index;
        }
        if list.tail == to_visit {
          if let Some(previous) = previous {
            list.tail = previous;
          } else {
            list.tail = u32::MAX;
          }
        }
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

  /// visitor (data, index) return should continue
  pub fn visit(&self, list: &ListHandle, mut visitor: impl FnMut(&T, u32) -> bool) {
    let mut next_to_visit = IndexPtr::new((list.head != u32::MAX).then_some(list.head as usize));
    while let Some(to_visit) = next_to_visit.get() {
      let to_visit = to_visit as u32;
      let data = self.pool.get(to_visit);
      next_to_visit = data.next;
      let should_continue = visitor(&data.data, to_visit);

      if !should_continue {
        return;
      }
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct ListHandle {
  head: u32,
  tail: u32,
}

impl ListHandle {
  pub fn is_empty(&self) -> bool {
    self.head == u32::MAX
  }
}

impl Default for ListHandle {
  fn default() -> Self {
    Self {
      head: u32::MAX,
      tail: u32::MAX,
    }
  }
}

struct LinkListNode<T> {
  next: IndexPtr,
  data: T,
}

#[test]
fn test() {
  let mut pool = LinkListPool::default();
  let mut list_a = ListHandle::default();
  let mut list_b = ListHandle::default();

  let a_1 = pool.insert(&mut list_a, 0);
  let a_2 = pool.insert(&mut list_a, 1);

  pool.remove(&mut list_a, a_1);
  assert_eq!(pool.list_len(&mut list_a), 1);
  pool.remove(&mut list_a, a_2);
  assert_eq!(pool.list_len(&mut list_a), 0);

  let _ = pool.insert(&mut list_a, 2);
  assert_eq!(pool.list_len(&mut list_a), 1);
  let _ = pool.insert(&mut list_a, 2);
  assert_eq!(pool.list_len(&mut list_a), 2);
  let a_5 = pool.insert(&mut list_a, 1);
  assert_eq!(pool.list_len(&mut list_a), 3);

  pool.visit_and_remove(&mut list_a, |v, _| (*v == 2, true));
  assert_eq!(pool.list_len(&mut list_a), 1);
  pool.remove(&mut list_a, a_5);
  assert_eq!(pool.list_len(&mut list_a), 0);

  let _ = pool.insert(&mut list_b, 1);
  assert_eq!(pool.list_len(&mut list_b), 1);
  pool.drop_list(&mut list_b);
}
