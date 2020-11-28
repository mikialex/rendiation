use std::fmt::Debug;

use crate::{Arithmetic, Math};

pub trait Scalar: Copy + Arithmetic + Math + Debug {}
impl Scalar for f32 {}
impl Scalar for f64 {}
