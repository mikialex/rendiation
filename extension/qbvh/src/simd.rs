use std::ops::{Add, Mul, Sub};

use rendiation_geometry::{Box3, IntersectAble, Ray3};

use crate::*;

/// we use a new type to avoid our algebra crate to be depended on simba
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Vec3ForSimd<T> {
  pub x: T,
  pub y: T,
  pub z: T,
}

impl<T: Add<Output = T>> Add for Vec3ForSimd<T> {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Vec3ForSimd {
      x: self.x + rhs.x,
      y: self.y + rhs.y,
      z: self.z + rhs.z,
    }
  }
}
impl<T: Sub<Output = T>> Sub for Vec3ForSimd<T> {
  type Output = Self;

  fn sub(self, rhs: Self) -> Self::Output {
    Vec3ForSimd {
      x: self.x - rhs.x,
      y: self.y - rhs.y,
      z: self.z - rhs.z,
    }
  }
}
impl<T: Mul<Output = T> + Copy> Mul<T> for Vec3ForSimd<T> {
  type Output = Self;

  fn mul(self, rhs: T) -> Self::Output {
    Vec3ForSimd {
      x: self.x * rhs,
      y: self.y * rhs,
      z: self.z * rhs,
    }
  }
}

impl<T> Functor for Vec3ForSimd<T> {
  type Unwrapped = T;
  type Wrapped<B> = Vec3ForSimd<B>;

  fn f_map<F, B>(self, mut f: F) -> Self::Wrapped<B>
  where
    F: FnMut(Self::Unwrapped) -> B,
  {
    Vec3ForSimd {
      x: f(self.x),
      y: f(self.y),
      z: f(self.z),
    }
  }

  fn f_filter_map<F, B>(self, mut f: F) -> Option<Self::Wrapped<B>>
  where
    F: FnMut(Self::Unwrapped) -> Option<B>,
  {
    Vec3ForSimd {
      x: f(self.x)?,
      y: f(self.y)?,
      z: f(self.z)?,
    }
    .into()
  }
}

impl<T> From<Vec3<T>> for Vec3ForSimd<T> {
  fn from(value: Vec3<T>) -> Self {
    Vec3ForSimd {
      x: value.x,
      y: value.y,
      z: value.z,
    }
  }
}

impl<T> Vec3ForSimd<T> {
  #[inline]
  fn map<F, U>(self, f: F) -> Vec3ForSimd<U>
  where
    F: Fn(T) -> U,
  {
    Vec3ForSimd {
      x: f(self.x),
      y: f(self.y),
      z: f(self.z),
    }
  }

  #[inline]
  fn zip<F, T2, U>(self, v2: Vec3ForSimd<T2>, f: F) -> Vec3ForSimd<U>
  where
    F: Fn(T, T2) -> U,
  {
    Vec3ForSimd {
      x: f(self.x, v2.x),
      y: f(self.y, v2.y),
      z: f(self.z, v2.z),
    }
  }
}

impl<T: One + Zero + Copy + SimdValue> SimdValue for Vec3ForSimd<T>
where
  T::Element: One + Zero + Copy,
{
  type Element = Vec3ForSimd<T::Element>;
  type SimdBool = T::SimdBool;

  #[inline(always)]
  fn lanes() -> usize {
    T::lanes()
  }

  #[inline(always)]
  fn splat(val: Self::Element) -> Self {
    val.map(T::splat)
  }

  #[inline(always)]
  fn extract(&self, i: usize) -> Self::Element {
    self.map(|e| e.extract(i))
  }

  #[inline(always)]
  unsafe fn extract_unchecked(&self, i: usize) -> Self::Element {
    self.map(|e| e.extract_unchecked(i))
  }

  #[inline(always)]
  fn replace(&mut self, i: usize, val: Self::Element) {
    *self = self.zip(val, |mut a, b| {
      a.replace(i, b);
      a
    })
  }

  #[inline(always)]
  unsafe fn replace_unchecked(&mut self, i: usize, val: Self::Element) {
    *self = self.zip(val, |mut a, b| {
      a.replace_unchecked(i, b);
      a
    })
  }

  #[inline(always)]
  fn select(self, cond: Self::SimdBool, other: Self) -> Self {
    self.zip(other, |a, b| a.select(cond, b))
  }
}

macro_rules! impl_simd_vector(
    ($VectorN: ident,$SimdVectorN:ident,$($field:ident),+) => {
        pub type $SimdVectorN = $VectorN<SimdRealValue>;
        impl From<[$VectorN<f32>; SIMD_WIDTH]> for $SimdVectorN {
            fn from(value:[$VectorN<f32>; SIMD_WIDTH])->Self{
                $VectorN{ $($field:array!(|i| value[i].$field;SIMD_WIDTH).into(),)+}
            }
        }
    }
);
impl_simd_vector!(Vec3ForSimd, SimdVec3, x, y, z);

impl From<SimdHyperAABB<Vec3ForSimd<SimdRealValue>>> for HyperAABBForSimd<Vec3ForSimd<f32>> {
  fn from(value: SimdHyperAABB<Vec3ForSimd<SimdRealValue>>) -> Self {
    value.to_merged_aabb()
  }
}

impl CenterAblePrimitive for HyperAABBForSimd<Vec3ForSimd<f32>> {
  type Center = Vec3<f32>;

  fn get_center(&self) -> Self::Center {
    let center = (self.max + self.min) * 0.5;
    Vec3::new(center.x, center.y, center.z)
  }
}

// this only act as the lane part of SimdHyperAABB;
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct HyperAABBForSimd<V> {
  pub min: V,
  pub max: V,
}

impl<V> HyperAABBForSimd<V> {
  pub fn new(min: V, max: V) -> Self {
    Self { min, max }
  }
  #[inline(always)]
  pub fn empty<T>() -> Self
  where
    T: Scalar,
    V: Vector<T>,
  {
    Self::new(
      Vector::splat(T::infinity()),
      Vector::splat(T::neg_infinity()),
    )
  }
}

impl<V: Copy> SimdValue for HyperAABBForSimd<V> {
  type Element = HyperAABBForSimd<V>;
  type SimdBool = bool;

  #[inline(always)]
  fn lanes() -> usize {
    1
  }

  #[inline(always)]
  fn splat(val: Self::Element) -> Self {
    val
  }

  #[inline(always)]
  fn extract(&self, _: usize) -> Self::Element {
    *self
  }

  #[inline(always)]
  unsafe fn extract_unchecked(&self, _: usize) -> Self::Element {
    *self
  }

  #[inline(always)]
  fn replace(&mut self, _: usize, val: Self::Element) {
    *self = val
  }

  #[inline(always)]
  unsafe fn replace_unchecked(&mut self, _: usize, val: Self::Element) {
    *self = val
  }

  #[inline(always)]
  fn select(self, cond: Self::SimdBool, other: Self) -> Self {
    if cond {
      self
    } else {
      other
    }
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimdHyperAABB<V> {
  pub mins: V,
  pub maxs: V,
}

impl<V> SimdHyperAABB<V> {
  /// An invalid Aabb.
  #[inline(always)]
  pub fn empty<S, T>() -> Self
  where
    S: Scalar,
    T: Vector<S>,
    V: SimdValue<Element = T>,
  {
    Self::splat(HyperAABBForSimd::<T>::empty())
  }

  /// Expand current simd aabb with another simd aabb
  #[inline(always)]
  pub fn expand_by_other<T>(&mut self, other: Self)
  where
    T: SimdPartialOrd + One + Zero + Copy,
    V: Vector<T>,
  {
    self.mins = self.mins.zip(other.mins, |a, b| a.simd_min(b));
    self.maxs = self.maxs.zip(other.maxs, |a, b| a.simd_max(b));
  }

  #[inline(always)]
  pub fn union<T>(&mut self, other: Self)
  where
    T: SimdPartialOrd + One + Zero + Copy,
    V: Vector<T>,
  {
    self.expand_by_other(other)
  }

  /// Merge all the Aabb represented by `self` into a single one.
  #[inline(always)]
  pub fn to_merged_aabb<T>(&self) -> HyperAABBForSimd<FunctorMapped<V, T::Element>>
  where
    T: SimdPartialOrd,
    V: Copy + Functor<Unwrapped = T>,
  {
    HyperAABBForSimd::new(
      self.mins.f_map(|e| e.simd_horizontal_min()),
      self.maxs.f_map(|e| e.simd_horizontal_max()),
    )
  }
}

impl<V> SimdValue for SimdHyperAABB<V>
where
  V: SimdValue,
  V::Element: Copy,
{
  type Element = HyperAABBForSimd<V::Element>;
  type SimdBool = V::SimdBool;

  #[inline(always)]
  fn lanes() -> usize {
    V::lanes()
  }

  #[inline(always)]
  fn splat(val: Self::Element) -> Self {
    Self {
      mins: V::splat(val.min),
      maxs: V::splat(val.max),
    }
  }

  #[inline(always)]
  fn extract(&self, lane: usize) -> Self::Element {
    HyperAABBForSimd::new(self.mins.extract(lane), self.maxs.extract(lane))
  }

  #[inline(always)]
  unsafe fn extract_unchecked(&self, lane: usize) -> Self::Element {
    HyperAABBForSimd::new(
      self.mins.extract_unchecked(lane),
      self.maxs.extract_unchecked(lane),
    )
  }

  #[inline(always)]
  fn replace(&mut self, i: usize, val: Self::Element) {
    self.mins.replace(i, val.min);
    self.maxs.replace(i, val.max);
  }

  #[inline(always)]
  unsafe fn replace_unchecked(&mut self, i: usize, val: Self::Element) {
    self.mins.replace_unchecked(i, val.min);
    self.maxs.replace_unchecked(i, val.max);
  }

  #[inline(always)]
  fn select(self, cond: Self::SimdBool, other: Self) -> Self {
    let mins = self.mins.select(cond, other.mins);
    let maxs = self.maxs.select(cond, other.maxs);
    Self { mins, maxs }
  }
}

pub type SimdBox3 = SimdHyperAABB<Vec3ForSimd<SimdRealValue>>;

impl SimdBox3 {
  /// The half-extents of all the Aabbs represented by `self``.
  pub fn half_extents(&self) -> Vec3<SimdRealValue> {
    let r = (self.maxs - self.mins) * SimdRealValue::splat(0.5);
    Vec3::new(r.x, r.y, r.z)
  }

  /// Enlarges this bounding volume by the given margin.
  #[inline]
  pub fn loosen(&mut self, margin: SimdRealValue) {
    let margins = Vec3ForSimd {
      x: margin,
      y: margin,
      z: margin,
    };
    self.mins = self.mins - margins;
    self.maxs = self.maxs + margins;
  }

  /// Lanewise check which Aabb represented by `self` contains the given set of `other` aabbs.
  /// The check is performed lane-wise.
  /// Note: we can not adapt this method to Containable trait for now, because data type Scalar
  /// is not compatible with SimdReal
  pub fn contains(&self, other: &Self) -> SimdBoolValue {
    self.mins.x.simd_le(other.mins.x)
      & self.mins.y.simd_le(other.mins.y)
      & self.mins.z.simd_le(other.mins.z)
      & self.maxs.x.simd_ge(other.maxs.x)
      & self.maxs.y.simd_ge(other.maxs.y)
      & self.maxs.z.simd_ge(other.maxs.z)
  }

  /// Check which Aabb represented by `self` contains the given `point`.
  pub fn contains_point(&self, point: &Vec3<SimdRealValue>) -> SimdBoolValue {
    self.mins.x.simd_le(point.x)
      & self.mins.y.simd_le(point.y)
      & self.mins.z.simd_le(point.z)
      & self.maxs.x.simd_ge(point.x)
      & self.maxs.y.simd_ge(point.y)
      & self.maxs.z.simd_ge(point.z)
  }

  pub fn intersects(&self, other: &Self) -> SimdBoolValue {
    self.mins.x.simd_le(other.maxs.x)
      & other.mins.x.simd_le(self.maxs.x)
      & self.mins.y.simd_le(other.maxs.y)
      & other.mins.y.simd_le(self.maxs.y)
      & self.mins.z.simd_le(other.maxs.z)
      & other.mins.z.simd_le(self.maxs.z)
  }

  pub fn equals(&self, other: &Self) -> SimdBoolValue {
    self.mins.x.simd_eq(other.mins.x)
      & self.mins.y.simd_eq(other.mins.y)
      & self.mins.z.simd_eq(other.mins.z)
      & self.maxs.x.simd_eq(other.maxs.x)
      & self.maxs.y.simd_eq(other.maxs.y)
      & self.maxs.z.simd_eq(other.maxs.z)
  }

  /// Casts a ray on all the Aabbs represented by `self`.
  pub fn intersect_ray(
    &self,
    ray: &Ray3,
    max_toi: SimdRealValue,
  ) -> (SimdBoolValue, SimdRealValue) {
    let one = SimdRealValue::one();
    let mut tmin = SimdRealValue::zero();
    let mut tmax = max_toi;

    let ray_origin = <Vec3ForSimd<SimdRealValue> as SimdValue>::splat(ray.origin.into());
    let ray_dir = <Vec3ForSimd<SimdRealValue> as SimdValue>::splat(ray.direction.value.into());
    let mut each_dimension = |min: SimdRealValue,
                              max: SimdRealValue,
                              dir_comp: SimdRealValue,
                              origin_comp: SimdRealValue| {
      let denom = one / dir_comp;
      let near_bound = (min - origin_comp) * denom;
      let far_bound = (max - origin_comp) * denom;

      tmin = tmin.simd_max(near_bound.simd_min(far_bound));
      tmax = tmax.simd_min(near_bound.simd_max(far_bound));
    };
    each_dimension(self.mins.x, self.maxs.x, ray_dir.x, ray_origin.x);
    each_dimension(self.mins.y, self.maxs.y, ray_dir.y, ray_origin.y);
    each_dimension(self.mins.z, self.maxs.z, ray_dir.z, ray_origin.z);

    (tmin.simd_le(tmax), tmin)
  }
}

impl From<[HyperAABBForSimd<Vec3ForSimd<f32>>; SIMD_WIDTH]> for SimdBox3 {
  fn from(aabbs: [HyperAABBForSimd<Vec3ForSimd<f32>>; SIMD_WIDTH]) -> Self {
    let mins = array![|ii| aabbs[ii].min; SIMD_WIDTH];
    let maxs = array![|ii| aabbs[ii].max; SIMD_WIDTH];

    SimdHyperAABB {
      mins: Vec3ForSimd {
        x: SimdRealValue::from([mins[0].x, mins[1].x, mins[2].x, mins[3].x]),
        y: SimdRealValue::from([mins[0].y, mins[1].y, mins[2].y, mins[3].y]),
        z: SimdRealValue::from([mins[0].z, mins[1].z, mins[2].z, mins[3].z]),
      },
      maxs: Vec3ForSimd {
        x: SimdRealValue::from([maxs[0].x, maxs[1].x, maxs[2].x, maxs[3].x]),
        y: SimdRealValue::from([maxs[0].y, maxs[1].y, maxs[2].y, maxs[3].y]),
        z: SimdRealValue::from([maxs[0].z, maxs[1].z, maxs[2].z, maxs[3].z]),
      },
    }
  }
}

// intersect_reverse!(Ray3, SimdBool, (), SimdBox3);
impl IntersectAble<Ray3, SimdBoolValue> for SimdBox3 {
  fn intersect(&self, other: &Ray3, _param: &()) -> SimdBoolValue {
    self.intersect_ray(other, SimdRealValue::splat(f32::MAX)).0
  }
}

// intersect_reverse!(Box3, SimdBool, (), SimdBox3);
impl IntersectAble<Box3, SimdBoolValue> for SimdBox3 {
  fn intersect(&self, other: &Box3, _param: &()) -> SimdBoolValue {
    let other = SimdBox3::splat(box3_to_simd_ver(*other));
    self.intersects(&other)
  }
}

pub fn box3_to_simd_ver(box3: Box3) -> HyperAABBForSimd<Vec3ForSimd<f32>> {
  HyperAABBForSimd {
    min: box3.min.into(),
    max: box3.max.into(),
  }
}

#[test]
fn test_simd_aabb() {
  let aabbs = [
    box3_to_simd_ver(Box3::new(vec3(0., 0., 0.), vec3(1., 1., 1.))),
    box3_to_simd_ver(Box3::new(vec3(1., 1., 1.), vec3(2., 2., 2.))),
    box3_to_simd_ver(Box3::new(vec3(2., 2., 2.), vec3(3., 3., 3.))),
    box3_to_simd_ver(Box3::new(vec3(3., 3., 3.), vec3(4., 4., 4.))),
  ];
  let mut simd_aabb: SimdBox3 = aabbs.into();
  let merged = simd_aabb.to_merged_aabb();
  assert_eq!(
    merged,
    box3_to_simd_ver(Box3::new(vec3(0., 0., 0.), vec3(4., 4., 4.)))
  );

  simd_aabb.loosen([1., 2., 3., 4.].into());
  assert_eq!(
    simd_aabb.extract(0),
    box3_to_simd_ver(Box3::new(vec3(-1., -1., -1.), vec3(2., 2., 2.)))
  );
  assert_eq!(
    simd_aabb.extract(1),
    box3_to_simd_ver(Box3::new(vec3(-1., -1., -1.), vec3(4., 4., 4.)))
  );
  assert_eq!(
    simd_aabb.extract(2),
    box3_to_simd_ver(Box3::new(vec3(-1., -1., -1.), vec3(6., 6., 6.)))
  );
  assert_eq!(
    simd_aabb.extract(3),
    box3_to_simd_ver(Box3::new(vec3(-1., -1., -1.), vec3(8., 8., 8.)))
  );
}

#[test]
fn test_simd_aabb_intersect() {
  let aabbs = [
    box3_to_simd_ver(Box3::empty()),
    box3_to_simd_ver(Box3::new(vec3(0., 0., 0.), vec3(2., 2., 2.))),
    box3_to_simd_ver(Box3::new(vec3(2., 2., 2.), vec3(3., 3., 3.))),
    box3_to_simd_ver(Box3::new(vec3(3., 3., 3.), vec3(4., 4., 4.))),
  ];
  let simd_aabb: SimdBox3 = aabbs.into();
  let ray = Ray3::from_origin_to_target(vec3(3., 1., 1.), vec3(0., 1., 1.));

  // an empty box report a hit as well.
  assert_eq!(
    simd_aabb
      .intersect_ray(&ray, SimdRealValue::splat(f32::MAX))
      .0,
    SimdBoolValue::from([true, true, false, false])
  );
}
