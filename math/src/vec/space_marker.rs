#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Space<T, S> {
  value: T,
  space_marker: PhantomData<S>,
}

impl<T, S> Deref for Space<T, S> {
  type Target = T;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.value
  }
}
impl<T, S> DerefMut for Space<T, S> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.value
  }
}
