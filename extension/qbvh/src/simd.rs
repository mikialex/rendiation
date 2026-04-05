use crate::*;

/// we use a new type with minimal impl to avoid our algebra crate to be depended on simba
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

  const LANES: usize = T::LANES;

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

pub type SimdVec3 = Vec3ForSimd<SimdRealValue>;
impl From<[Vec3ForSimd<f32>; SIMD_WIDTH]> for SimdVec3 {
  fn from(value: [Vec3ForSimd<f32>; SIMD_WIDTH]) -> Self {
    Vec3ForSimd {
      x: array!(|i|value[i].x;
            SIMD_WIDTH)
      .into(),
      y: array!(|i|value[i].y;
            SIMD_WIDTH)
      .into(),
      z: array!(|i|value[i].z;
            SIMD_WIDTH)
      .into(),
    }
  }
}

pub type Box3ForSimd = HyperAABBForSimd<Vec3ForSimd<f32>>;

impl From<HyperAABBForSimd<Vec3ForSimd<SimdRealValue>>> for Box3ForSimd {
  fn from(value: HyperAABBForSimd<Vec3ForSimd<SimdRealValue>>) -> Self {
    value.to_merged_aabb()
  }
}

impl CenterAblePrimitive for Box3ForSimd {
  type Center = Vec3<f32>;

  fn get_center(&self) -> Self::Center {
    let center = (self.max + self.min) * 0.5;
    Vec3::new(center.x, center.y, center.z)
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HyperAABBForSimd<V> {
  pub min: V,
  pub max: V,
}

impl<V> HyperAABBForSimd<V> {
  /// An invalid Aabb.
  #[inline(always)]
  pub fn empty<S, T>() -> Self
  where
    S: Scalar,
    // T: Vector<S>,
    V: SimdValue<Element = S>,
  {
    Self {
      min: V::splat(S::infinity()),
      max: V::splat(S::neg_infinity()),
    }
  }

  /// Expand current simd aabb with another simd aabb
  #[inline(always)]
  pub fn expand_by_other<T>(&mut self, other: Self)
  where
    T: SimdPartialOrd + One + Zero + Copy,
    V: Vector<T>,
  {
    self.min = self.min.zip(other.min, |a, b| a.simd_min(b));
    self.max = self.max.zip(other.max, |a, b| a.simd_max(b));
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
    HyperAABBForSimd {
      min: self.min.f_map(|e| e.simd_horizontal_min()),
      max: self.max.f_map(|e| e.simd_horizontal_max()),
    }
  }
}

impl<V> SimdValue for HyperAABBForSimd<V>
where
  V: SimdValue,
  V::Element: Copy,
{
  type Element = HyperAABBForSimd<V::Element>;
  type SimdBool = V::SimdBool;

  const LANES: usize = V::LANES;

  #[inline(always)]
  fn splat(val: Self::Element) -> Self {
    Self {
      min: V::splat(val.min),
      max: V::splat(val.max),
    }
  }

  #[inline(always)]
  fn extract(&self, lane: usize) -> Self::Element {
    HyperAABBForSimd {
      min: self.min.extract(lane),
      max: self.max.extract(lane),
    }
  }

  #[inline(always)]
  unsafe fn extract_unchecked(&self, lane: usize) -> Self::Element {
    HyperAABBForSimd {
      min: self.min.extract_unchecked(lane),
      max: self.max.extract_unchecked(lane),
    }
  }

  #[inline(always)]
  fn replace(&mut self, i: usize, val: Self::Element) {
    self.min.replace(i, val.min);
    self.max.replace(i, val.max);
  }

  #[inline(always)]
  unsafe fn replace_unchecked(&mut self, i: usize, val: Self::Element) {
    self.min.replace_unchecked(i, val.min);
    self.max.replace_unchecked(i, val.max);
  }

  #[inline(always)]
  fn select(self, cond: Self::SimdBool, other: Self) -> Self {
    let mins = self.min.select(cond, other.min);
    let maxs = self.max.select(cond, other.max);
    Self {
      min: mins,
      max: maxs,
    }
  }
}

pub type SimdBox3 = HyperAABBForSimd<SimdVec3>;

impl SimdBox3 {
  /// The half-extents of all the Aabbs represented by `self``.
  pub fn half_extents(&self) -> Vec3<SimdRealValue> {
    let r = (self.max - self.min) * SimdRealValue::splat(0.5);
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
    self.min = self.min - margins;
    self.max = self.max + margins;
  }

  /// Lanewise check which Aabb represented by `self` contains the given set of `other` aabbs.
  /// The check is performed lane-wise.
  /// Note: we can not adapt this method to Containable trait for now, because data type Scalar
  /// is not compatible with SimdReal
  pub fn contains(&self, other: &Self) -> SimdBoolValue {
    self.min.x.simd_le(other.min.x)
      & self.min.y.simd_le(other.min.y)
      & self.min.z.simd_le(other.min.z)
      & self.max.x.simd_ge(other.max.x)
      & self.max.y.simd_ge(other.max.y)
      & self.max.z.simd_ge(other.max.z)
  }

  /// Check which Aabb represented by `self` contains the given `point`.
  pub fn contains_point(&self, point: &Vec3<SimdRealValue>) -> SimdBoolValue {
    self.min.x.simd_le(point.x)
      & self.min.y.simd_le(point.y)
      & self.min.z.simd_le(point.z)
      & self.max.x.simd_ge(point.x)
      & self.max.y.simd_ge(point.y)
      & self.max.z.simd_ge(point.z)
  }

  pub fn intersects(&self, other: &Self) -> SimdBoolValue {
    self.min.x.simd_le(other.max.x)
      & other.min.x.simd_le(self.max.x)
      & self.min.y.simd_le(other.max.y)
      & other.min.y.simd_le(self.max.y)
      & self.min.z.simd_le(other.max.z)
      & other.min.z.simd_le(self.max.z)
  }

  pub fn equals(&self, other: &Self) -> SimdBoolValue {
    self.min.x.simd_eq(other.min.x)
      & self.min.y.simd_eq(other.min.y)
      & self.min.z.simd_eq(other.min.z)
      & self.max.x.simd_eq(other.max.x)
      & self.max.y.simd_eq(other.max.y)
      & self.max.z.simd_eq(other.max.z)
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
    each_dimension(self.min.x, self.max.x, ray_dir.x, ray_origin.x);
    each_dimension(self.min.y, self.max.y, ray_dir.y, ray_origin.y);
    each_dimension(self.min.z, self.max.z, ray_dir.z, ray_origin.z);

    (tmin.simd_le(tmax), tmin)
  }
}

impl From<[Box3ForSimd; SIMD_WIDTH]> for SimdBox3 {
  fn from(aabbs: [Box3ForSimd; SIMD_WIDTH]) -> Self {
    let mins = array![|ii| aabbs[ii].min; SIMD_WIDTH];
    let maxs = array![|ii| aabbs[ii].max; SIMD_WIDTH];

    HyperAABBForSimd {
      min: Vec3ForSimd {
        x: SimdRealValue::from([mins[0].x, mins[1].x, mins[2].x, mins[3].x]),
        y: SimdRealValue::from([mins[0].y, mins[1].y, mins[2].y, mins[3].y]),
        z: SimdRealValue::from([mins[0].z, mins[1].z, mins[2].z, mins[3].z]),
      },
      max: Vec3ForSimd {
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
    let other = SimdBox3::splat(box3_to_box3_for_simd(*other));
    self.intersects(&other)
  }
}

pub fn box3_to_box3_for_simd(box3: Box3) -> Box3ForSimd {
  HyperAABBForSimd {
    min: box3.min.into(),
    max: box3.max.into(),
  }
}

#[test]
fn test_simd_aabb() {
  let aabbs = [
    box3_to_box3_for_simd(Box3::new(vec3(0., 0., 0.), vec3(1., 1., 1.))),
    box3_to_box3_for_simd(Box3::new(vec3(1., 1., 1.), vec3(2., 2., 2.))),
    box3_to_box3_for_simd(Box3::new(vec3(2., 2., 2.), vec3(3., 3., 3.))),
    box3_to_box3_for_simd(Box3::new(vec3(3., 3., 3.), vec3(4., 4., 4.))),
  ];
  let mut simd_aabb: SimdBox3 = aabbs.into();
  let merged = simd_aabb.to_merged_aabb();
  assert_eq!(
    merged,
    box3_to_box3_for_simd(Box3::new(vec3(0., 0., 0.), vec3(4., 4., 4.)))
  );

  simd_aabb.loosen([1., 2., 3., 4.].into());
  assert_eq!(
    simd_aabb.extract(0),
    box3_to_box3_for_simd(Box3::new(vec3(-1., -1., -1.), vec3(2., 2., 2.)))
  );
  assert_eq!(
    simd_aabb.extract(1),
    box3_to_box3_for_simd(Box3::new(vec3(-1., -1., -1.), vec3(4., 4., 4.)))
  );
  assert_eq!(
    simd_aabb.extract(2),
    box3_to_box3_for_simd(Box3::new(vec3(-1., -1., -1.), vec3(6., 6., 6.)))
  );
  assert_eq!(
    simd_aabb.extract(3),
    box3_to_box3_for_simd(Box3::new(vec3(-1., -1., -1.), vec3(8., 8., 8.)))
  );
}

#[test]
fn test_simd_aabb_intersect() {
  let aabbs = [
    box3_to_box3_for_simd(Box3::empty()),
    box3_to_box3_for_simd(Box3::new(vec3(0., 0., 0.), vec3(2., 2., 2.))),
    box3_to_box3_for_simd(Box3::new(vec3(2., 2., 2.), vec3(3., 3., 3.))),
    box3_to_box3_for_simd(Box3::new(vec3(3., 3., 3.), vec3(4., 4., 4.))),
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
