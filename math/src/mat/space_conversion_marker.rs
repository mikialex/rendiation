#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct SpaceConversionMatrix<T, From, To> {
  value: T,
  from_space: PhantomData<From>,
  to_space: PhantomData<To>,
}

impl<T, From, To> SpaceConversionMatrix<T, From, To> {
  //
}
