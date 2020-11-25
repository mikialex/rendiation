use std::{mem, mem::MaybeUninit, ops::*};

use crate::Vector;

// try specialization
impl<A: Copy, B: Copy> Add<Vector<B, 2>> for Vector<A, 2>
where
  A: Add<B>,
{
  fn add(self, rhs: Vector<B, 2>) -> Self::Output {
    Vector([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
  }
}

impl<A, B, const N: usize> Add<Vector<B, { N }>> for Vector<A, { N }>
where
  A: Add<B>,
{
  type Output = Vector<<A as Add<B>>::Output, { N }>;

  default fn add(self, rhs: Vector<B, { N }>) -> Self::Output {
    let mut sum = MaybeUninit::<[<A as Add<B>>::Output; N]>::uninit();
    let mut lhs = MaybeUninit::new(self);
    let mut rhs = MaybeUninit::new(rhs);
    let sump: *mut <A as Add<B>>::Output = unsafe { mem::transmute(&mut sum) };
    let lhsp: *mut MaybeUninit<A> = unsafe { mem::transmute(&mut lhs) };
    let rhsp: *mut MaybeUninit<B> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..N {
      unsafe {
        sump.add(i).write(
          lhsp.add(i).replace(MaybeUninit::uninit()).assume_init()
            + rhsp.add(i).replace(MaybeUninit::uninit()).assume_init(),
        );
      }
    }
    Vector::<<A as Add<B>>::Output, { N }>(unsafe { sum.assume_init() })
  }
}

impl<A, B, const N: usize> AddAssign<Vector<B, { N }>> for Vector<A, { N }>
where
  A: AddAssign<B>,
{
  fn add_assign(&mut self, rhs: Vector<B, { N }>) {
    let mut rhs = MaybeUninit::new(rhs);
    let rhsp: *mut MaybeUninit<B> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..N {
      self.0[i] += unsafe { rhsp.add(i).replace(MaybeUninit::uninit()).assume_init() };
    }
  }
}

impl<A, B, const N: usize> Sub<Vector<B, { N }>> for Vector<A, { N }>
where
  A: Sub<B>,
{
  type Output = Vector<<A as Sub<B>>::Output, { N }>;

  fn sub(self, rhs: Vector<B, { N }>) -> Self::Output {
    let mut dif = MaybeUninit::<[<A as Sub<B>>::Output; N]>::uninit();
    let mut lhs = MaybeUninit::new(self);
    let mut rhs = MaybeUninit::new(rhs);
    let difp: *mut <A as Sub<B>>::Output = unsafe { mem::transmute(&mut dif) };
    let lhsp: *mut MaybeUninit<A> = unsafe { mem::transmute(&mut lhs) };
    let rhsp: *mut MaybeUninit<B> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..N {
      unsafe {
        difp.add(i).write(
          lhsp.add(i).replace(MaybeUninit::uninit()).assume_init()
            - rhsp.add(i).replace(MaybeUninit::uninit()).assume_init(),
        );
      }
    }
    Vector::<<A as Sub<B>>::Output, { N }>(unsafe { dif.assume_init() })
  }
}

impl<A, B, const N: usize> SubAssign<Vector<B, { N }>> for Vector<A, { N }>
where
  A: SubAssign<B>,
{
  fn sub_assign(&mut self, rhs: Vector<B, { N }>) {
    let mut rhs = MaybeUninit::new(rhs);
    let rhsp: *mut MaybeUninit<B> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..N {
      self.0[i] -= unsafe { rhsp.add(i).replace(MaybeUninit::uninit()).assume_init() };
    }
  }
}

impl<T, const N: usize> Neg for Vector<T, { N }>
where
  T: Neg,
{
  type Output = Vector<<T as Neg>::Output, { N }>;

  fn neg(self) -> Self::Output {
    let mut from = MaybeUninit::new(self);
    let mut neg = MaybeUninit::<[<T as Neg>::Output; N]>::uninit();
    let fromp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut from) };
    let negp: *mut <T as Neg>::Output = unsafe { mem::transmute(&mut neg) };
    for i in 0..N {
      unsafe {
        negp.add(i).write(
          fromp
            .add(i)
            .replace(MaybeUninit::uninit())
            .assume_init()
            .neg(),
        );
      }
    }
    Vector::<<T as Neg>::Output, { N }>(unsafe { neg.assume_init() })
  }
}

/// Scalar multiply
impl<A, B, const N: usize> Mul<B> for Vector<A, { N }>
where
  A: Mul<B>,
  B: Clone,
{
  type Output = Vector<<A as Mul<B>>::Output, { N }>;

  fn mul(self, scalar: B) -> Self::Output {
    let mut from = MaybeUninit::new(self);
    let mut scaled = MaybeUninit::<[<A as Mul<B>>::Output; N]>::uninit();
    let fromp: *mut MaybeUninit<A> = unsafe { mem::transmute(&mut from) };
    let scaledp: *mut <A as Mul<B>>::Output = unsafe { mem::transmute(&mut scaled) };
    for i in 0..N {
      unsafe {
        scaledp
          .add(i)
          .write(fromp.add(i).replace(MaybeUninit::uninit()).assume_init() * scalar.clone());
      }
    }
    Vector::<<A as Mul<B>>::Output, { N }>(unsafe { scaled.assume_init() })
  }
}

impl<const N: usize> Mul<Vector<f32, { N }>> for f32 {
  type Output = Vector<f32, { N }>;

  fn mul(self, vec: Vector<f32, { N }>) -> Self::Output {
    vec * self
  }
}

impl<const N: usize> Mul<Vector<f64, { N }>> for f64 {
  type Output = Vector<f64, { N }>;

  fn mul(self, vec: Vector<f64, { N }>) -> Self::Output {
    vec * self
  }
}

/// Scalar multiply assign
impl<A, B, const N: usize> MulAssign<B> for Vector<A, { N }>
where
  A: MulAssign<B>,
  B: Clone,
{
  fn mul_assign(&mut self, scalar: B) {
    for i in 0..N {
      self.0[i] *= scalar.clone();
    }
  }
}

/// Scalar divide
impl<A, B, const N: usize> Div<B> for Vector<A, { N }>
where
  A: Div<B>,
  B: Clone,
{
  type Output = Vector<<A as Div<B>>::Output, { N }>;

  fn div(self, scalar: B) -> Self::Output {
    let mut from = MaybeUninit::new(self);
    let mut scaled = MaybeUninit::<[<A as Div<B>>::Output; N]>::uninit();
    let fromp: *mut MaybeUninit<A> = unsafe { mem::transmute(&mut from) };
    let scaledp: *mut <A as Div<B>>::Output = unsafe { mem::transmute(&mut scaled) };
    for i in 0..N {
      unsafe {
        scaledp
          .add(i)
          .write(fromp.add(i).replace(MaybeUninit::uninit()).assume_init() / scalar.clone());
      }
    }
    Vector::<<A as Div<B>>::Output, { N }>(unsafe { scaled.assume_init() })
  }
}

/// Scalar divide assign
impl<A, B, const N: usize> DivAssign<B> for Vector<A, { N }>
where
  A: DivAssign<B>,
  B: Clone,
{
  fn div_assign(&mut self, scalar: B) {
    for i in 0..N {
      self.0[i] /= scalar.clone();
    }
  }
}
