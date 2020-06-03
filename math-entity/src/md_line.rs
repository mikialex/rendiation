struct MultiDimensionalLine<T, U>{
  pub normal: T,
  pub constant: U,
}

trait DimensionOne{}

pub struct Successor<T> where T: Nat {
  _marker: PhantomData<T>,
}

pub trait DimensionSuccessor<T>{
  fn downgrade(&self) -> T;
}

impl<T> DimensionSuccessor<T> for T{
  
}