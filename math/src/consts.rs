pub trait One: Sized { fn one() -> Self; }
pub trait Zero: Sized { fn zero() -> Self; }
pub trait Two: Sized { fn two() -> Self; }
pub trait UnitX: Sized { fn unit_x() -> Self; }
pub trait UnitY: Sized { fn unit_y() -> Self; }
pub trait UnitZ: Sized { fn unit_z() -> Self; }
pub trait UnitW: Sized { fn unit_w() -> Self; }
pub trait OneHalf: Sized { fn onehalf() -> Self; }
pub trait Pi: Sized { fn pi() -> Self; }
pub trait Pi2: Sized { fn pi2() -> Self; }
pub trait Pi4: Sized { fn pi4() -> Self; }
pub trait Pi8: Sized { fn pi8() -> Self; }
pub trait PiByC180: Sized { fn pi_by_c180() -> Self; }
pub trait C180ByPi: Sized { fn c180_by_pi() -> Self; }
pub trait Epsilon: Sized { fn epsilon() -> Self; }

impl One for f32      { #[inline(always)] fn one() -> Self { 1.0_f32 } }
impl Zero for f32     { #[inline(always)] fn zero() -> Self { 0.0_f32 } }
impl Two for f32      { #[inline(always)] fn two() -> Self { 2.0_f32 } }
impl OneHalf for f32  { #[inline(always)] fn onehalf() -> Self { 0.5_f32 } }
impl Pi for f32       { #[inline(always)] fn pi() -> Self { std::f32::consts::PI } }
impl Pi2 for f32      { #[inline(always)] fn pi2() -> Self { Self::pi() * 2.0_f32 } }
impl Pi4 for f32      { #[inline(always)] fn pi4() -> Self { Self::pi() * 4.0_f32 } }
impl Pi8 for f32      { #[inline(always)] fn pi8() -> Self { Self::pi() * 8.0_f32 } }
impl PiByC180 for f32 { #[inline(always)] fn pi_by_c180() -> Self { std::f32::consts::PI / 180.0_f32 } }
impl C180ByPi for f32 { #[inline(always)] fn c180_by_pi() -> Self { 180.0_f32 / std::f32::consts::PI } }
impl Epsilon for f32  { #[inline(always)] fn epsilon() -> Self { 0.00001_f32 } }

impl One for f64      { #[inline(always)] fn one() -> Self { 1.0_f64 } }
impl Zero for f64     { #[inline(always)] fn zero() -> Self { 0.0_f64 } }
impl Two for f64      { #[inline(always)] fn two() -> Self { 2.0_f64 } }
impl OneHalf for f64  { #[inline(always)] fn onehalf() -> Self { 0.5_f64 } }
impl Pi for f64       { #[inline(always)] fn pi() -> Self { std::f64::consts::PI } }
impl Pi2 for f64      { #[inline(always)] fn pi2() -> Self { Self::pi() * 2.0_f64 } }
impl Pi4 for f64      { #[inline(always)] fn pi4() -> Self { Self::pi() * 4.0_f64 } }
impl Pi8 for f64      { #[inline(always)] fn pi8() -> Self { Self::pi() * 8.0_f64 } }
impl PiByC180 for f64 { #[inline(always)] fn pi_by_c180() -> Self { Self::pi() / 180.0_f64 } }
impl C180ByPi for f64 { #[inline(always)] fn c180_by_pi() -> Self { 180.0_f64 / Self::pi() } }
impl Epsilon for f64  { #[inline(always)] fn epsilon() -> Self { 0.000_000_01_f64 } }