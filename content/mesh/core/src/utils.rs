/// https://internals.rust-lang.org/t/pre-rfc-tryfromiterator-and-try-collect-to-enable-collecting-to-arrays/14423
pub trait TryFromIterator<A>: Sized {
  type Error;

  fn try_from_iter<T: IntoIterator<Item = A>>(iter: T) -> Result<Self, Self::Error>;
}

impl<X, A> TryFromIterator<A> for X
where
  X: FromIterator<A>,
{
  type Error = ();

  fn try_from_iter<T: IntoIterator<Item = A>>(iter: T) -> Result<Self, Self::Error> {
    Ok(Self::from_iter(iter))
  }
}

/// Abstract over containers that could get item by index and by value.
pub trait IndexGet {
  type Output;
  fn index_get(&self, key: usize) -> Option<Self::Output>;
}

impl<T: Copy> IndexGet for Vec<T> {
  type Output = T;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    self.get(key).copied()
  }
}

impl<T: Copy> IndexGet for &[T] {
  type Output = T;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    self.get(key).copied()
  }
}

/// Abstract over containers that could get size.
pub trait CollectionSize {
  fn len(&self) -> usize;

  fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

impl<T> CollectionSize for Vec<T> {
  fn len(&self) -> usize {
    self.len()
  }
}

impl<T> CollectionSize for &[T] {
  fn len(&self) -> usize {
    (*self).len()
  }
}
