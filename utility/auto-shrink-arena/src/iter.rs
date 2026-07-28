use core::iter::{self, Extend, FromIterator, FusedIterator};
use core::slice;
use std::marker::PhantomData;
use std::vec;

use crate::*;

#[derive(Clone, Debug)]
pub struct IntoIter<T> {
  pub(crate) len: usize,
  pub(crate) inner: vec::IntoIter<Option<(u64, T)>>,
}

impl<T> Iterator for IntoIter<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some(None) => continue,
        Some(Some((_, value))) => {
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
        Some(None) => continue,
        Some(Some((_, value))) => {
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

impl<'a, T> IntoIterator for &'a AutoShrinkArena<T> {
  type Item = (Handle<T>, &'a T);
  type IntoIter = Iter<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, T: 'a> {
  pub(crate) len: usize,
  pub(crate) inner: iter::Enumerate<slice::Iter<'a, Option<(u64, T)>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
  type Item = (Handle<T>, &'a T);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some((_, None)) => continue,
        Some((handle, Some((generation, value)))) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation: *generation,
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

impl<T> DoubleEndedIterator for Iter<'_, T> {
  fn next_back(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next_back() {
        Some((_, None)) => continue,
        Some((handle, Some((generation, value)))) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation: *generation,
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

impl<T> ExactSizeIterator for Iter<'_, T> {
  fn len(&self) -> usize {
    self.len
  }
}

impl<T> FusedIterator for Iter<'_, T> {}

impl<'a, T> IntoIterator for &'a mut AutoShrinkArena<T> {
  type Item = (Handle<T>, &'a mut T);
  type IntoIter = IterMut<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}

#[derive(Debug)]
pub struct IterMut<'a, T: 'a> {
  pub(crate) len: usize,
  pub(crate) inner: iter::Enumerate<slice::IterMut<'a, Option<(u64, T)>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
  type Item = (Handle<T>, &'a mut T);

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next() {
        Some((_, None)) => continue,
        Some((handle, Some((generation, value)))) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation: *generation,
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

impl<T> DoubleEndedIterator for IterMut<'_, T> {
  fn next_back(&mut self) -> Option<Self::Item> {
    loop {
      match self.inner.next_back() {
        Some((_, None)) => continue,
        Some((handle, Some((generation, value)))) => {
          self.len -= 1;
          let idx = Handle {
            handle,
            generation: *generation,
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

impl<T> ExactSizeIterator for IterMut<'_, T> {
  fn len(&self) -> usize {
    self.len
  }
}

impl<T> FusedIterator for IterMut<'_, T> {}

impl<T> Extend<T> for AutoShrinkArena<T> {
  fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    for t in iter {
      self.insert(t);
    }
  }
}

impl<T> FromIterator<T> for AutoShrinkArena<T> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let iter = iter.into_iter();
    let (lower, upper) = iter.size_hint();
    let cap = upper.unwrap_or(lower);
    let cap = cmp::max(cap, 1);
    let mut arena = AutoShrinkArena::with_capacity(cap);
    arena.extend(iter);
    arena
  }
}
