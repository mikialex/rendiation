use core::cmp;
use core::mem;
use core::ops;
use std::marker::PhantomData;

mod handle;
pub use handle::*;
mod iter;
pub use iter::*;

#[derive(Clone, Debug)]
pub struct AutoShrinkArena<T> {
  items: Vec<Option<(u64, T)>>,
  low_free: Vec<usize>,
  high_free: Vec<usize>,
  len: usize,
  next_generation: u64,
}

const DEFAULT_CAPACITY: usize = 0;

impl<T> Default for AutoShrinkArena<T> {
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let arena = AutoShrinkArena::<i32>::default();
  /// assert_eq!(arena.len(), 0);
  /// ```
  fn default() -> AutoShrinkArena<T> {
    AutoShrinkArena::new()
  }
}

impl<T> AutoShrinkArena<T> {
  /// Constructs a new, empty `AutoShrinkArena`.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::<usize>::new();
  /// # let _ = arena;
  /// ```
  pub fn new() -> AutoShrinkArena<T> {
    AutoShrinkArena::with_capacity(DEFAULT_CAPACITY)
  }

  pub fn memory_usage_in_bytes(&self) -> usize {
    self.items.capacity() * mem::size_of::<Option<(u64, T)>>()
      + self.low_free.capacity() * mem::size_of::<usize>()
      + self.high_free.capacity() * mem::size_of::<usize>()
  }

  /// Constructs a new, empty `AutoShrinkArena<T>` with the specified capacity.
  ///
  /// The `AutoShrinkArena<T>` will be able to hold `n` elements without further allocation.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::with_capacity(10);
  ///
  /// // These insertions will not require further allocation.
  /// for i in 0..10 {
  ///   assert!(arena.try_insert(i).is_ok());
  /// }
  ///
  /// // But now we are at capacity, and there is no more room.
  /// assert!(arena.try_insert(99).is_err());
  /// ```
  pub fn with_capacity(n: usize) -> AutoShrinkArena<T> {
    let n = cmp::max(n, 1);
    let mut arena = AutoShrinkArena {
      items: Vec::new(),
      low_free: Vec::new(),
      high_free: Vec::new(),
      len: 0,
      next_generation: 0,
    };
    arena.reserve(n);
    arena
  }

  /// Clear all the items inside the arena, but keep its allocation.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::with_capacity(1);
  /// arena.insert(42);
  /// arena.insert(43);
  ///
  /// arena.clear();
  ///
  /// assert_eq!(arena.capacity(), 2);
  /// ```
  pub fn clear(&mut self) {
    for item in &mut self.items {
      *item = None;
    }
    if !self.is_empty() {
      self.next_generation += 1;
    }
    self.len = 0;
    self.rebuild_free_lists();
  }

  /// Attempts to insert `value` into the arena using existing capacity.
  ///
  /// This method will never allocate new capacity in the arena.
  ///
  /// If insertion succeeds, then the `value`'s handle is returned. If
  /// insertion fails, then `Err(value)` is returned to give ownership of
  /// `value` back to the caller.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  ///
  /// match arena.try_insert(42) {
  ///   Ok(idx) => {
  ///     assert_eq!(arena[idx], 42);
  ///   }
  ///   Err(x) => {
  ///     assert_eq!(x, 42);
  ///   }
  /// };
  /// ```
  #[inline]
  pub fn try_insert(&mut self, value: T) -> Result<Handle<T>, T> {
    match self.try_alloc_next_index() {
      None => Err(value),
      Some((index, generation)) => {
        self.items[index] = Some((generation, value));
        Ok(Handle {
          handle: index,
          generation,
          phantom: PhantomData,
        })
      }
    }
  }

  /// Attempts to insert the value returned by `create` into the arena using existing capacity.
  /// `create` is called with the new value's associated handle, allowing values that know their own
  /// handle.
  ///
  /// This method will never allocate new capacity in the arena.
  ///
  /// If insertion succeeds, then the new handle is returned. If
  /// insertion fails, then `Err(create)` is returned to give ownership of
  /// `create` back to the caller.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::{AutoShrinkArena, Handle};
  ///
  /// let mut arena = AutoShrinkArena::new();
  ///
  /// match arena.try_insert_with(|idx| (42, idx.into_raw_parts().0)) {
  ///   Ok(idx) => {
  ///     assert_eq!(arena[idx].0, 42);
  ///     assert_eq!(arena[idx].1, idx.into_raw_parts().0);
  ///   }
  ///   Err(x) => {
  ///     // Insertion failed.
  ///   }
  /// };
  /// ```
  #[inline]
  pub fn try_insert_with<F: FnOnce(Handle<T>) -> T>(&mut self, create: F) -> Result<Handle<T>, F> {
    match self.try_alloc_next_index() {
      None => Err(create),
      Some((index, generation)) => {
        let (raw_handle, raw_gen) = (index, generation);
        let value = create(Handle {
          handle: raw_handle,
          generation: raw_gen,
          phantom: PhantomData,
        });
        self.items[index] = Some((raw_gen, value));
        Ok(Handle {
          handle: raw_handle,
          generation: raw_gen,
          phantom: PhantomData,
        })
      }
    }
  }

  #[inline]
  fn try_alloc_next_index(&mut self) -> Option<(usize, u64)> {
    // prefer low_free, then high_free
    if let Some(i) = self.low_free.pop() {
      self.len += 1;
      return Some((i, self.next_generation));
    }
    if let Some(i) = self.high_free.pop() {
      self.len += 1;
      return Some((i, self.next_generation));
    }
    None
  }

  /// Insert `value` into the arena, allocating more capacity if necessary.
  ///
  /// The `value`'s associated handle in the arena is returned.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  ///
  /// let idx = arena.insert(42);
  /// assert_eq!(arena[idx], 42);
  /// ```
  #[inline]
  pub fn insert(&mut self, value: T) -> Handle<T> {
    match self.try_insert(value) {
      Ok(i) => i,
      Err(value) => self.insert_slow_path(value),
    }
  }

  /// Insert the value returned by `create` into the arena, allocating more capacity if necessary.
  /// `create` is called with the new value's associated handle, allowing values that know their own
  /// handle.
  ///
  /// The new value's associated handle in the arena is returned.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::{AutoShrinkArena, Handle};
  ///
  /// let mut arena = AutoShrinkArena::new();
  ///
  /// let idx = arena.insert_with(|idx| (42, idx.into_raw_parts().0));
  /// assert_eq!(arena[idx].0, 42);
  /// assert_eq!(arena[idx].1, idx.into_raw_parts().0);
  /// ```
  #[inline]
  pub fn insert_with(&mut self, create: impl FnOnce(Handle<T>) -> T) -> Handle<T> {
    match self.try_insert_with(create) {
      Ok(i) => i,
      Err(create) => self.insert_with_slow_path(create),
    }
  }

  #[inline(never)]
  fn insert_slow_path(&mut self, value: T) -> Handle<T> {
    let len = if self.capacity() == 0 {
      1
    } else {
      self.items.len()
    };
    self.reserve(len);
    self
      .try_insert(value)
      .map_err(|_| ())
      .expect("inserting will always succeed after reserving additional space")
  }

  #[inline(never)]
  fn insert_with_slow_path(&mut self, create: impl FnOnce(Handle<T>) -> T) -> Handle<T> {
    let len = self.items.len();
    self.reserve(len);
    self
      .try_insert_with(create)
      .map_err(|_| ())
      .expect("inserting will always succeed after reserving additional space")
  }

  /// Remove the element at handle `i` from the arena.
  ///
  /// If the element at handle `i` is still in the arena, then it is
  /// returned. If it is not in the arena, then `None` is returned.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// let idx = arena.insert(42);
  ///
  /// assert_eq!(arena.remove(idx), Some(42));
  /// assert_eq!(arena.remove(idx), None);
  /// ```
  pub fn remove(&mut self, i: Handle<T>) -> Option<T> {
    if i.handle >= self.items.len() {
      return None;
    }

    match &mut self.items[i.handle] {
      Some((generation, _)) if *generation == i.generation => {
        let slot = self.items[i.handle].take();
        self.next_generation += 1;
        self.len -= 1;

        let mid = self.items.len() / 2;
        if i.handle < mid {
          self.low_free.push(i.handle);
        } else {
          self.high_free.push(i.handle);
        }

        self.try_auto_shrink();

        match slot {
          Some((_, value)) => Some(value),
          _ => unreachable!(),
        }
      }
      _ => None,
    }
  }

  /// Retains only the elements specified by the predicate.
  ///
  /// In other words, remove all indices such that `predicate(handle, &value)` returns `false`.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut crew = AutoShrinkArena::new();
  /// crew.extend(&[
  ///   "Jim Hawkins",
  ///   "John Silver",
  ///   "Alexander Smollett",
  ///   "Israel Hands",
  /// ]);
  /// let pirates = ["John Silver", "Israel Hands"];
  /// crew.retain(|_index, member| !pirates.contains(member));
  /// let mut crew_members = crew.iter().map(|(_, member)| **member);
  /// assert_eq!(crew_members.next(), Some("Jim Hawkins"));
  /// assert_eq!(crew_members.next(), Some("Alexander Smollett"));
  /// assert!(crew_members.next().is_none());
  /// ```
  pub fn retain(&mut self, mut predicate: impl FnMut(Handle<T>, &mut T) -> bool) {
    for i in 0..self.capacity() {
      let should_remove = match &mut self.items[i] {
        Some((generation, value)) => {
          let handle = Handle {
            handle: i,
            generation: *generation,
            phantom: PhantomData,
          };
          !predicate(handle, value)
        }
        _ => false,
      };
      if should_remove {
        let handle = Handle {
          handle: i,
          generation: self.items[i].as_ref().unwrap().0,
          phantom: PhantomData,
        };
        self.remove(handle);
      }
    }
  }

  /// Is the element at handle `i` in the arena?
  ///
  /// Returns `true` if the element at `i` is in the arena, `false` otherwise.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// let idx = arena.insert(42);
  ///
  /// assert!(arena.contains(idx));
  /// arena.remove(idx);
  /// assert!(!arena.contains(idx));
  /// ```
  pub fn contains(&self, i: Handle<T>) -> bool {
    self.get(i).is_some()
  }

  /// Get a shared reference to the element at handle `i` if it is in the
  /// arena.
  ///
  /// If the element at handle `i` is not in the arena, then `None` is returned.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// let idx = arena.insert(42);
  ///
  /// assert_eq!(arena.get(idx), Some(&42));
  /// arena.remove(idx);
  /// assert!(arena.get(idx).is_none());
  /// ```
  pub fn get(&self, i: Handle<T>) -> Option<&T> {
    match self.items.get(i.handle) {
      Some(Some((generation, value))) if *generation == i.generation => Some(value),
      _ => None,
    }
  }

  /// Get an exclusive reference to the element at handle `i` if it is in the
  /// arena.
  ///
  /// If the element at handle `i` is not in the arena, then `None` is returned.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// let idx = arena.insert(42);
  ///
  /// *arena.get_mut(idx).unwrap() += 1;
  /// assert_eq!(arena.remove(idx), Some(43));
  /// assert!(arena.get_mut(idx).is_none());
  /// ```
  pub fn get_mut(&mut self, i: Handle<T>) -> Option<&mut T> {
    match self.items.get_mut(i.handle) {
      Some(Some((generation, value))) if *generation == i.generation => Some(value),
      _ => None,
    }
  }

  /// Get a pair of exclusive references to the elements at handle `i1` and `i2` if it is in the
  /// arena.
  ///
  /// If the element at handle `i1` or `i2` is not in the arena, then `None` is returned for this
  /// element.
  ///
  /// # Panics
  ///
  /// Panics if `i1` and `i2` are pointing to the same item of the arena.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// let idx1 = arena.insert(0);
  /// let idx2 = arena.insert(1);
  ///
  /// {
  ///   let (item1, item2) = arena.get2_mut(idx1, idx2);
  ///   *item1.unwrap() = 3;
  ///   *item2.unwrap() = 4;
  /// }
  ///
  /// assert_eq!(arena[idx1], 3);
  /// assert_eq!(arena[idx2], 4);
  /// ```
  pub fn get2_mut(&mut self, i1: Handle<T>, i2: Handle<T>) -> (Option<&mut T>, Option<&mut T>) {
    let len = self.items.len();

    if i1.handle == i2.handle {
      assert!(i1.generation != i2.generation);

      if i1.generation > i2.generation {
        return (self.get_mut(i1), None);
      }
      return (None, self.get_mut(i2));
    }

    if i1.handle >= len {
      return (None, self.get_mut(i2));
    } else if i2.handle >= len {
      return (self.get_mut(i1), None);
    }

    let (raw_item1, raw_item2) = {
      let (xs, ys) = self.items.split_at_mut(cmp::max(i1.handle, i2.handle));
      if i1.handle < i2.handle {
        (&mut xs[i1.handle], &mut ys[0])
      } else {
        (&mut ys[0], &mut xs[i2.handle])
      }
    };

    let item1 = match raw_item1 {
      Some((generation, value)) if *generation == i1.generation => Some(value),
      _ => None,
    };

    let item2 = match raw_item2 {
      Some((generation, value)) if *generation == i2.generation => Some(value),
      _ => None,
    };

    (item1, item2)
  }

  /// Get the length of this arena.
  ///
  /// The length is the number of elements the arena holds.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// assert_eq!(arena.len(), 0);
  ///
  /// let idx = arena.insert(42);
  /// assert_eq!(arena.len(), 1);
  ///
  /// let _ = arena.insert(0);
  /// assert_eq!(arena.len(), 2);
  ///
  /// assert_eq!(arena.remove(idx), Some(42));
  /// assert_eq!(arena.len(), 1);
  /// ```
  pub fn len(&self) -> usize {
    self.len
  }

  /// Returns true if the arena contains no elements
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// assert!(arena.is_empty());
  ///
  /// let idx = arena.insert(42);
  /// assert!(!arena.is_empty());
  ///
  /// assert_eq!(arena.remove(idx), Some(42));
  /// assert!(arena.is_empty());
  /// ```
  pub fn is_empty(&self) -> bool {
    self.len == 0
  }

  /// Get the capacity of this arena.
  ///
  /// The capacity is the maximum number of elements the arena can hold
  /// without further allocation, including however many it currently
  /// contains.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::with_capacity(10);
  /// assert_eq!(arena.capacity(), 10);
  ///
  /// // `try_insert` does not allocate new capacity.
  /// for i in 0..10 {
  ///   assert!(arena.try_insert(1).is_ok());
  ///   assert_eq!(arena.capacity(), 10);
  /// }
  ///
  /// // But `insert` will if the arena is already at capacity.
  /// arena.insert(0);
  /// assert!(arena.capacity() > 10);
  /// ```
  pub fn capacity(&self) -> usize {
    self.items.len()
  }

  /// Allocate space for `additional_capacity` more elements in the arena.
  ///
  /// # Panics
  ///
  /// Panics if this causes the capacity to overflow.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::with_capacity(10);
  /// arena.reserve(5);
  /// assert_eq!(arena.capacity(), 15);
  /// # let _: AutoShrinkArena<usize> = arena;
  /// ```
  pub fn reserve(&mut self, additional_capacity: usize) {
    let start = self.items.len();
    let end = self.items.len() + additional_capacity;
    self.items.reserve_exact(additional_capacity);
    self.items.extend((start..end).map(|_| None));
    self.rebuild_free_lists();
  }

  /// Iterate over shared references to the elements in this arena.
  ///
  /// Yields pairs of `(Handle, &T)` items.
  ///
  /// Order of iteration is not defined.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// for i in 0..10 {
  ///   arena.insert(i * i);
  /// }
  ///
  /// for (idx, value) in arena.iter() {
  ///   println!("{} is at handle {:?}", value, idx);
  /// }
  /// ```
  pub fn iter(&self) -> Iter<'_, T> {
    Iter {
      len: self.len,
      inner: self.items.iter().enumerate(),
    }
  }

  /// Iterate over exclusive references to the elements in this arena.
  ///
  /// Yields pairs of `(Handle, &mut T)` items.
  ///
  /// Order of iteration is not defined.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// for i in 0..10 {
  ///   arena.insert(i * i);
  /// }
  ///
  /// for (_idx, value) in arena.iter_mut() {
  ///   *value += 5;
  /// }
  /// ```
  pub fn iter_mut(&mut self) -> IterMut<'_, T> {
    IterMut {
      len: self.len,
      inner: self.items.iter_mut().enumerate(),
    }
  }

  /// Iterate over elements of the arena and remove them.
  ///
  /// Yields pairs of `(Handle, T)` items.
  ///
  /// Order of iteration is not defined.
  ///
  /// Note: All elements are removed even if the iterator is only partially consumed or not consumed
  /// at all.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// let idx_1 = arena.insert("hello");
  /// let idx_2 = arena.insert("world");
  ///
  /// assert!(arena.get(idx_1).is_some());
  /// assert!(arena.get(idx_2).is_some());
  /// for (idx, value) in arena.drain() {
  ///   assert!((idx == idx_1 && value == "hello") || (idx == idx_2 && value == "world"));
  /// }
  /// assert!(arena.get(idx_1).is_none());
  /// assert!(arena.get(idx_2).is_none());
  /// ```
  pub fn drain(&mut self) -> Drain<'_, T> {
    let old_len = self.len;
    if !self.is_empty() {
      self.next_generation += 1;
    }
    self.low_free.clear();
    self.high_free.clear();
    self.len = 0;
    Drain {
      len: old_len,
      inner: self.items.drain(..).enumerate(),
    }
  }

  /// Given an i of `usize` without a generation, get a shared reference
  /// to the element and the matching `Handle` of the entry behind `i`.
  ///
  /// This method is useful when you know there might be an element at the
  /// position i, but don't know its generation or precise Handle.
  ///
  /// Use cases include using indexing such as Hierarchical BitMap Indexing or
  /// other kinds of bit-efficient indexing.
  ///
  /// You should use the `get` method instead most of the time.
  pub fn get_unknown_gen(&self, i: usize) -> Option<(&T, Handle<T>)> {
    match self.items.get(i) {
      Some(Some((generation, value))) => Some((
        value,
        Handle {
          generation: *generation,
          handle: i,
          phantom: PhantomData,
        },
      )),
      _ => None,
    }
  }

  /// Given an i of `usize` without a generation, get an exclusive reference
  /// to the element and the matching `Handle` of the entry behind `i`.
  ///
  /// This method is useful when you know there might be an element at the
  /// position i, but don't know its generation or precise Handle.
  ///
  /// Use cases include using indexing such as Hierarchical BitMap Indexing or
  /// other kinds of bit-efficient indexing.
  ///
  /// You should use the `get_mut` method instead most of the time.
  pub fn get_unknown_gen_mut(&mut self, i: usize) -> Option<(&mut T, Handle<T>)> {
    match self.items.get_mut(i) {
      Some(Some((generation, value))) => Some((
        value,
        Handle {
          generation: *generation,
          handle: i,
          phantom: PhantomData,
        },
      )),
      _ => None,
    }
  }

  /// Get the given position's alive handle, if the given position out of bounds or do not
  /// have alive value, the None will be returned
  pub fn get_handle(&self, index: usize) -> Option<Handle<T>> {
    match self.items.get(index) {
      Some(Some((generation, _))) => Some(Handle {
        handle: index,
        generation: *generation,
        phantom: PhantomData,
      }),
      _ => None,
    }
  }

  /// Attempt to shrink the arena by truncating trailing free slots.
  ///
  /// First checks if the entire high half (indices >= capacity/2) is empty
  /// via the high_free list (O(1)). If so, truncates to capacity/2.
  /// Otherwise, linearly scans from the end to find the last occupied slot
  /// and truncates there.
  ///
  /// # Examples
  ///
  /// ```
  /// use auto_shrink_arena::AutoShrinkArena;
  ///
  /// let mut arena = AutoShrinkArena::new();
  /// for i in 0..20 {
  ///   arena.insert(i);
  /// }
  ///
  /// // Remove everything — high half becomes empty, auto-shrinks
  /// let handles: Vec<_> = arena.iter().map(|(h, _)| h).collect();
  /// for h in handles {
  ///   arena.remove(h);
  /// }
  ///
  /// // Arena has been auto-shrunk after removes; force a final shrink
  /// arena.shrink();
  /// assert_eq!(arena.capacity(), 1);
  /// ```
  pub fn shrink(&mut self) {
    let cap = self.items.len();
    if cap <= 1 {
      return;
    }

    let mid = cap / 2;
    let high_region_size = cap - mid;
    if self.high_free.len() == high_region_size {
      if self.is_empty() {
        self.do_truncate(1);
      } else {
        self.items.truncate(mid);
        self.items.shrink_to_fit();
        self.high_free.clear();
        self.high_free.shrink_to_fit();
      }
    } else {
      match self.items.iter().rposition(|x| x.is_some()) {
        Some(last_occupied) => {
          let new_len = cmp::max(last_occupied + 1, 1);
          if new_len < cap {
            self.do_truncate(new_len);
          }
        }
        None => {
          self.do_truncate(1);
        }
      }
    }
  }

  /// Called after each remove: only shrinks if the entire high half is empty.
  /// O(1) check, safe to call frequently.
  fn try_auto_shrink(&mut self) {
    let cap = self.items.len();
    if cap <= 1 {
      return;
    }
    let mid = cap / 2;
    let high_region_size = cap - mid;
    if self.high_free.len() == high_region_size {
      if self.is_empty() {
        self.do_truncate(1);
      } else {
        self.items.truncate(mid);
        self.items.shrink_to_fit();
        self.high_free.clear();
        self.high_free.shrink_to_fit();
      }
    }
  }

  fn do_truncate(&mut self, new_len: usize) {
    self.items.truncate(new_len);
    self.items.shrink_to_fit();
    self.rebuild_free_lists();
  }

  fn rebuild_free_lists(&mut self) {
    self.low_free.clear();
    self.high_free.clear();
    let mid = self.items.len() / 2;
    for (i, slot) in self.items.iter().enumerate() {
      if slot.is_none() {
        if i < mid {
          self.low_free.push(i);
        } else {
          self.high_free.push(i);
        }
      }
    }
    self.low_free.shrink_to_fit();
    self.high_free.shrink_to_fit();
  }
}

impl<T> IntoIterator for AutoShrinkArena<T> {
  type Item = T;
  type IntoIter = IntoIter<T>;
  fn into_iter(self) -> Self::IntoIter {
    IntoIter {
      len: self.len,
      inner: self.items.into_iter(),
    }
  }
}

impl<T> ops::Index<Handle<T>> for AutoShrinkArena<T> {
  type Output = T;

  fn index(&self, handle: Handle<T>) -> &Self::Output {
    self.get(handle).expect("No element at handle")
  }
}

impl<T> ops::IndexMut<Handle<T>> for AutoShrinkArena<T> {
  fn index_mut(&mut self, handle: Handle<T>) -> &mut Self::Output {
    self.get_mut(handle).expect("No element at handle")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Verify: every slot is either occupied (not in any free list) or free
  /// (in exactly the correct free list based on the mid threshold).
  fn check_invariants<T>(arena: &AutoShrinkArena<T>) {
    let mid = arena.capacity() / 2;
    for (i, slot) in arena.items.iter().enumerate() {
      if slot.is_some() {
        assert!(!arena.low_free.contains(&i));
        assert!(!arena.high_free.contains(&i));
      } else {
        let in_low = arena.low_free.contains(&i);
        let in_high = arena.high_free.contains(&i);
        assert!(
          in_low ^ in_high,
          "free slot {} should be in exactly one free list (low={}, high={})",
          i,
          in_low,
          in_high
        );
        if i < mid {
          assert!(in_low, "slot {} < mid={} should be in low_free", i, mid);
        } else {
          assert!(in_high, "slot {} >= mid={} should be in high_free", i, mid);
        }
      }
    }
    // len must be consistent: total occupied + total free == capacity
    let occupied = arena.items.iter().filter(|s| s.is_some()).count();
    assert_eq!(occupied, arena.len, "len field should match occupied count");
    assert_eq!(
      occupied + arena.low_free.len() + arena.high_free.len(),
      arena.capacity(),
      "occupied + low_free + high_free should equal capacity"
    );
  }

  #[test]
  fn reserve_then_shrink_then_insert() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::new();
    let cap_before = arena.capacity();
    arena.reserve(10);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), cap_before + 10);

    arena.shrink();
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 1);

    arena.reserve(9);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 10);
    let handles: Vec<_> = (0..10).map(|i| arena.insert(i)).collect();
    check_invariants(&arena);
    assert_eq!(arena.len(), 10);
    for (i, h) in handles.iter().enumerate() {
      assert_eq!(arena[*h], i as i32);
    }
  }

  #[test]
  fn multiple_reserve_calls() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::new();
    let initial_cap = arena.capacity();
    arena.reserve(5);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), initial_cap + 5);
    arena.reserve(5);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), initial_cap + 10);
    arena.reserve(10);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), initial_cap + 20);

    for i in 0..arena.capacity() {
      assert!(arena.try_insert(i as i32).is_ok());
    }
    check_invariants(&arena);
    assert!(arena.try_insert(999).is_err());
  }

  #[test]
  fn multiple_shrink_calls() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::with_capacity(100);
    let handles: Vec<_> = (0..100).map(|i| arena.insert(i)).collect();
    check_invariants(&arena);

    let to_remove: Vec<_> = handles.iter().filter(|h| h.handle >= 50).copied().collect();
    for h in &to_remove {
      arena.remove(*h);
    }
    check_invariants(&arena);
    assert!(arena.capacity() <= 50);

    for i in 100..150 {
      arena.insert(i);
    }
    check_invariants(&arena);
    assert!(arena.capacity() >= 100);

    let all: Vec<_> = arena.iter().map(|(h, _)| h).collect();
    for h in all {
      arena.remove(h);
    }
    check_invariants(&arena);
    arena.shrink();
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 1);
    arena.shrink();
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 1);
  }

  #[test]
  fn reserve_after_partial_remove() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::with_capacity(20);
    let handles: Vec<_> = (0..20).map(|i| arena.insert(i)).collect();
    check_invariants(&arena);

    for h in handles.iter().filter(|h| h.handle % 3 == 0) {
      arena.remove(*h);
    }
    check_invariants(&arena);

    let cap_before_reserve = arena.capacity();
    arena.reserve(10);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), cap_before_reserve + 10);

    let occupied_before = arena.len();
    for _ in 0..10 {
      arena.insert(9999);
    }
    check_invariants(&arena);
    assert_eq!(arena.len(), occupied_before + 10);
  }

  #[test]
  fn shrink_preserves_generational_check() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::with_capacity(10);
    let handles: Vec<_> = (0..10).map(|i| arena.insert(i)).collect();

    let to_remove: Vec<_> = handles
      .iter()
      .filter(|h| h.handle >= arena.capacity() / 2)
      .copied()
      .collect();
    for h in &to_remove {
      arena.remove(*h);
    }
    check_invariants(&arena);

    let cap_after = arena.capacity();
    assert!(cap_after < 10);

    for h in &to_remove {
      assert!(arena.get(*h).is_none());
    }
    for h in handles.iter().filter(|h| h.handle < cap_after) {
      assert!(arena.get(*h).is_some());
    }

    arena.reserve(10);
    check_invariants(&arena);
    for _ in 0..5 {
      arena.insert(999);
    }
    check_invariants(&arena);
    for h in &to_remove {
      assert!(arena.get(*h).is_none());
    }
  }

  #[test]
  fn insert_remove_shrink_cycle() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::new();

    for cycle in 0..5 {
      let handles: Vec<_> = (0..20).map(|i| arena.insert(i + cycle * 100)).collect();
      check_invariants(&arena);
      assert_eq!(arena.len(), 20);

      for h in handles {
        arena.remove(h);
      }
      check_invariants(&arena);
      assert_eq!(arena.len(), 0);

      arena.shrink();
      check_invariants(&arena);
      assert_eq!(arena.capacity(), 1);
    }
  }

  #[test]
  fn remove_triggers_auto_shrink_only_when_high_half_empty() {
    let mut arena: AutoShrinkArena<i32> = AutoShrinkArena::with_capacity(20);
    let handles: Vec<_> = (0..20).map(|i| arena.insert(i)).collect();
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 20);

    let high_handle = *handles.iter().find(|h| h.handle >= 10).unwrap();
    arena.remove(high_handle);
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 20);

    let to_remove: Vec<_> = handles
      .iter()
      .filter(|h| h.handle >= 10 && **h != high_handle)
      .copied()
      .collect();
    for h in to_remove {
      arena.remove(h);
    }
    check_invariants(&arena);
    assert_eq!(arena.capacity(), 10);
  }
}
