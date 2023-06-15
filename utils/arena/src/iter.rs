use core::iter::{self, Extend, FromIterator, FusedIterator};

use crate::*;

/// An iterator over the elements in an arena.
///
/// Yields `T` items.
///
/// Order of iteration is not defined.
///
/// # Examples
///
/// ```
/// use arena::Arena;
///
/// let mut arena = Arena::new();
/// for i in 0..10 {
///   arena.insert(i * i);
/// }
///
/// for value in arena {
///   assert!(value < 100);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct IntoIter<T> {
  pub(crate) len: usize,
  pub(crate) inner: vec::IntoIter<Entry<T>>,
}

impl<T> Iterator for IntoIter<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some(Entry::Free { .. }) => continue,
        Some(Entry::Occupied { value, .. }) => {
          self.len -= 1;
          return Some(value);
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    (self.len, Some(self.len))
  }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
  fn next_back(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next_back() {
        Some(Entry::Free { .. }) => continue,
        Some(Entry::Occupied { value, .. }) => {
          self.len -= 1;
          return Some(value);
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }
}

impl<T> ExactSizeIterator for IntoIter<T> {
  fn len(&self) -> usize {
    self.len
  }
}

impl<T> FusedIterator for IntoIter<T> {}

impl<'a, T> IntoIterator for &'a Arena<T> {
  type Item = (Handle<T>, &'a T);
  type IntoIter = Iter<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

/// An iterator over shared references to the elements in an arena.
///
/// Yields pairs of `(Handle, &T)` items.
///
/// Order of iteration is not defined.
///
/// # Examples
///
/// ```
/// use arena::Arena;
///
/// let mut arena = Arena::new();
/// for i in 0..10 {
///   arena.insert(i * i);
/// }
///
/// for (idx, value) in &arena {
///   println!("{} is at handle {:?}", value, idx);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Iter<'a, T: 'a> {
  pub(crate) len: usize,
  pub(crate) inner: iter::Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
  type Item = (Handle<T>, &'a T);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some((_, &Entry::Free { .. })) => continue,
        Some((
          handle,
          &Entry::Occupied {
            generation,
            ref value,
          },
        )) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation,
            phantom: PhantomData,
          };
          return Some((idx, value));
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    (self.len, Some(self.len))
  }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
  fn next_back(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next_back() {
        Some((_, &Entry::Free { .. })) => continue,
        Some((
          handle,
          &Entry::Occupied {
            generation,
            ref value,
          },
        )) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation,
            phantom: PhantomData,
          };
          return Some((idx, value));
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
  fn len(&self) -> usize {
    self.len
  }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
  type Item = (Handle<T>, &'a mut T);
  type IntoIter = IterMut<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}

/// An iterator over exclusive references to elements in this arena.
///
/// Yields pairs of `(Handle, &mut T)` items.
///
/// Order of iteration is not defined.
///
/// # Examples
///
/// ```
/// use arena::Arena;
///
/// let mut arena = Arena::new();
/// for i in 0..10 {
///   arena.insert(i * i);
/// }
///
/// for (_idx, value) in &mut arena {
///   *value += 5;
/// }
/// ```
#[derive(Debug)]
pub struct IterMut<'a, T: 'a> {
  pub(crate) len: usize,
  pub(crate) inner: iter::Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
  type Item = (Handle<T>, &'a mut T);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some((_, &mut Entry::Free { .. })) => continue,
        Some((
          handle,
          &mut Entry::Occupied {
            generation,
            ref mut value,
          },
        )) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation,
            phantom: PhantomData,
          };
          return Some((idx, value));
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    (self.len, Some(self.len))
  }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
  fn next_back(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next_back() {
        Some((_, &mut Entry::Free { .. })) => continue,
        Some((
          handle,
          &mut Entry::Occupied {
            generation,
            ref mut value,
          },
        )) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation,
            phantom: PhantomData,
          };
          return Some((idx, value));
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
  fn len(&self) -> usize {
    self.len
  }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}

/// An iterator that removes elements from the arena.
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
/// use arena::Arena;
///
/// let mut arena = Arena::new();
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
#[derive(Debug)]
pub struct Drain<'a, T: 'a> {
  pub(crate) len: usize,
  pub(crate) inner: iter::Enumerate<vec::Drain<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Drain<'a, T> {
  type Item = (Handle<T>, T);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some((_, Entry::Free { .. })) => continue,
        Some((handle, Entry::Occupied { generation, value })) => {
          let idx = Handle {
            handle,
            generation,
            phantom: PhantomData,
          };
          self.len -= 1;
          return Some((idx, value));
        }
        None => {
          debug_assert_eq!(self.len, 0);
          return None;
        }
      }
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    (self.len, Some(self.len))
  }
}

impl<T> Extend<T> for Arena<T> {
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    for t in iter {
      self.insert(t);
    }
  }
}

impl<T> FromIterator<T> for Arena<T> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let iter = iter.into_iter();
    let (lower, upper) = iter.size_hint();
    let cap = upper.unwrap_or(lower);
    let cap = cmp::max(cap, 1);
    let mut arena = Arena::with_capacity(cap);
    arena.extend(iter);
    arena
  }
}
