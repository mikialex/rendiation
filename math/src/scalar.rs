use std::fmt::Debug;

use crate::Arithmetic;
use num_traits::real::Real;

pub trait Scalar: Copy + Arithmetic + Real + Debug {}
impl Scalar for f32 {}
impl Scalar for f64 {}
