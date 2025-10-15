use crate::*;

pub trait IntersectAbleAbstractMesh {
  fn intersect_list(&self, ray: Ray3, conf: &Config, result: &mut MeshBufferHitList);
  fn intersect_nearest(&self, ray: Ray3, conf: &Config) -> OptionalNearest<MeshBufferHitPoint>;
}

#[derive(Copy, Clone)]
pub struct MeshBufferHitPoint<T: Scalar = f32> {
  pub hit: HitPoint3D<T>,
  pub primitive_index: usize,
}
impl HitDistanceCompareAble for MeshBufferHitPoint {
  fn is_near_than(&self, other: &Self) -> bool {
    self.hit.is_near_than(&other.hit)
  }
}

pub struct MeshBufferHitList(pub Vec<MeshBufferHitPoint>);
impl MeshBufferHitList {
  pub fn new() -> Self {
    Self(Vec::new())
  }
}

impl Default for MeshBufferHitList {
  fn default() -> Self {
    Self::new()
  }
}

impl<G> IntersectAbleAbstractMesh for G
where
  G: AbstractMesh,
  G::Primitive: IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config>,
{
  fn intersect_list(&self, ray: Ray3, conf: &Config, result: &mut MeshBufferHitList) {
    self
      .primitive_iter()
      .enumerate()
      .filter_map(|(primitive_index, p)| {
        p.intersect(&ray, conf)
          .map(|hit| MeshBufferHitPoint {
            hit,
            primitive_index,
          })
          .0
      })
      .for_each(|h| result.0.push(h))
  }
  fn intersect_nearest(&self, ray: Ray3, conf: &Config) -> OptionalNearest<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    self
      .primitive_iter()
      .enumerate()
      .for_each(|(primitive_index, p)| {
        nearest.refresh_nearest(p.intersect(&ray, conf).map(|hit| MeshBufferHitPoint {
          hit,
          primitive_index,
        }));
      });
    nearest
  }
}

#[derive(Copy, Clone)]
pub enum ToleranceType {
  LocalSpace,
  ScreenSpace,
}

#[derive(Copy, Clone)]
pub struct IntersectTolerance {
  pub value: f32,
  pub ty: ToleranceType,
}
impl IntersectTolerance {
  pub fn new(value: f32, ty: ToleranceType) -> Self {
    Self { value, ty }
  }
}

#[derive(Clone)]
pub struct MeshBufferIntersectConfig {
  pub line_tolerance_local: f32,
  pub point_tolerance_local: f32,
  pub triangle_face: FaceSide,
}

impl Default for MeshBufferIntersectConfig {
  fn default() -> Self {
    Self {
      triangle_face: FaceSide::Double,
      line_tolerance_local: 0.05,
      point_tolerance_local: 0.05,
    }
  }
}

type Config = MeshBufferIntersectConfig;

impl<T> IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config> for Triangle<T>
where
  T: Positioned<Position = Vec3<f32>> + Copy,
{
  #[inline]
  fn intersect(&self, ray: &Ray3, c: &Config) -> OptionalNearest<HitPoint3D> {
    ray.intersect(self, &c.triangle_face)
  }
}

impl<T> IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config> for LineSegment<T>
where
  T: Positioned<Position = Vec3<f32>> + Copy,
{
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> OptionalNearest<HitPoint3D> {
    ray.intersect(self, &conf.line_tolerance_local)
  }
}

impl<T> IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config> for Point<T>
where
  T: Positioned<Position = Vec3<f32>> + Copy,
{
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> OptionalNearest<HitPoint3D> {
    ray.intersect(self, &conf.point_tolerance_local)
  }
}
