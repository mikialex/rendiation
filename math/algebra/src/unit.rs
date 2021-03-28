pub trait Unit {}

pub trait LengthUnit: Unit {}
pub trait AngleUnit: Unit {}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct UnitScalar<T, U: LengthUnit> {
  value: T,
  phantom: PhantomData<U>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct UnitAngle<T, U: AngleUnit> {
  value: T,
  phantom: PhantomData<U>,
}

pub struct Rad;
impl Unit for Rad {}
impl AngleUnit for Rad {}

pub struct Deg;
impl Unit for Deg {}
impl AngleUnit for Deg {}

pub type Rad<T> = UnitAngle<T, Rad>;
pub type Deg<T> = UnitAngle<T, Deg>;
