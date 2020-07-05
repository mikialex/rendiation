pub trait Unit {
}

pub struct UnitScalar<T, U: Unit>{
  value: T,
  phantom: PhantomData
}