use crate::{ArrayIter, Vector};
use core::fmt;
use std::{
  hash::{Hash, Hasher},
  iter::FromIterator,
  mem::{self, MaybeUninit},
  ops::{Deref, DerefMut},
};

impl<T, const N: usize> Clone for Vector<T, { N }>
where
  T: Clone,
{
  fn clone(&self) -> Self {
    Vector::<T, { N }>(self.0.clone())
  }
}

impl<T, const N: usize> Copy for Vector<T, { N }> where T: Copy {}

impl<A, B, RHS, const N: usize> PartialEq<RHS> for Vector<A, { N }>
where
  RHS: Deref<Target = [B; N]>,
  A: PartialEq<B>,
{
  fn eq(&self, other: &RHS) -> bool {
    for (a, b) in self.0.iter().zip(other.deref().iter()) {
      if !a.eq(b) {
        return false;
      }
    }
    true
  }
}

impl<T, const N: usize> Eq for Vector<T, { N }> where T: Eq {}

impl<T, const N: usize> Into<[T; N]> for Vector<T, { N }> {
  fn into(self) -> [T; N] {
    self.0
  }
}

impl<T, const N: usize> FromIterator<T> for Vector<T, { N }> {
  fn from_iter<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = T>,
  {
    let mut iter = iter.into_iter();
    let mut new = MaybeUninit::<Vector<T, { N }>>::uninit();
    let newp: *mut T = unsafe { mem::transmute(&mut new) };

    for i in 0..N {
      if let Some(next) = iter.next() {
        unsafe { newp.add(i).write(next) };
      } else {
        panic!("too few items in iterator to create Vector<_, {}>", N);
      }
    }

    if iter.next().is_some() {
      panic!("too many items in iterator to create Vector<_, {}>", N);
    }

    unsafe { new.assume_init() }
  }
}

impl<T, const N: usize> IntoIterator for Vector<T, { N }> {
  type Item = T;
  type IntoIter = ArrayIter<T, { N }>;

  fn into_iter(self) -> Self::IntoIter {
    let Vector(array) = self;
    ArrayIter {
      array: MaybeUninit::new(array),
      pos: 0,
    }
  }
}

impl<T, const N: usize> fmt::Debug for Vector<T, { N }>
where
  T: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match N {
      0 => unimplemented!(),
      1 => write!(f, "Vector {{ x: {:?} }}", self.0[0]),
      2 => write!(f, "Vector {{ x: {:?}, y: {:?} }}", self.0[0], self.0[1]),
      3 => write!(
        f,
        "Vector {{ x: {:?}, y: {:?}, z: {:?} }}",
        self.0[0], self.0[1], self.0[2]
      ),
      4 => write!(
        f,
        "Vector {{ x: {:?}, y: {:?}, z: {:?}, w: {:?} }}",
        self.0[0], self.0[1], self.0[2], self.0[3]
      ),
      _ => write!(
        f,
        "Vector {{ x: {:?}, y: {:?}, z: {:?}, w: {:?}, [..]: {:?} }}",
        self.0[0],
        self.0[1],
        self.0[2],
        self.0[3],
        &self.0[4..]
      ),
    }
  }
}

impl<T, const N: usize> Deref for Vector<T, { N }> {
  type Target = [T; N];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T, const N: usize> DerefMut for Vector<T, { N }> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T, const N: usize> Hash for Vector<T, { N }>
where
  T: Hash,
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    for i in 0..N {
      self.0[i].hash(state);
    }
  }
}
