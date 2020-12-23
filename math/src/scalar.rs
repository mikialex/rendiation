use std::fmt::Debug;

use crate::Arithmetic;
use num_traits::Float;

pub trait Scalar: Copy + Arithmetic + Debug + Float {}
impl Scalar for f32 {}
impl Scalar for f64 {}
