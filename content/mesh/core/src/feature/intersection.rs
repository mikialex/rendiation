use std::cell::Cell;

use crate::*;

pub trait IntersectAbleAbstractMesh {
  fn intersect_list(
    &self,
    ray: Ray3,
    conf: &Config,
    group: MeshGroup,
    result: &mut MeshBufferHitList,
  );
  fn intersect_nearest(
    &self,
    ray: Ray3,
    conf: &Config,
    group: MeshGroup,
  ) -> OptionalNearest<MeshBufferHitPoint>;
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
  G: AbstractMesh + GPUConsumableMeshBuffer,
  G::Primitive: IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config>,
{
  fn intersect_list(
    &self,
    ray: Ray3,
    conf: &Config,
    group: MeshGroup,
    result: &mut MeshBufferHitList,
  ) {
    self
      .primitive_iter_group(group)
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
  fn intersect_nearest(
    &self,
    ray: Ray3,
    conf: &Config,
    group: MeshGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    self
      .primitive_iter_group(group)
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

pub trait IntersectAbleGroupedMesh {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  );
  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint>;
}

impl<T> IntersectAbleGroupedMesh for GroupedMesh<T>
where
  T: IntersectAbleAbstractMesh + AbstractMesh + GPUConsumableMeshBuffer,
{
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &Config,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    let group = self.get_group(group);
    self.mesh.intersect_list(ray, conf, group, result)
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &Config,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    let group = self.get_group(group);
    self.mesh.intersect_nearest(ray, conf, group)
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
  pub line_tolerance: IntersectTolerance,
  pub point_tolerance: IntersectTolerance,
  pub current_item_scale_estimate: Cell<f32>,
}

impl Default for MeshBufferIntersectConfig {
  fn default() -> Self {
    Self {
      line_tolerance: IntersectTolerance::new(1.0, ToleranceType::LocalSpace),
      point_tolerance: IntersectTolerance::new(1.0, ToleranceType::LocalSpace),
      current_item_scale_estimate: Cell::new(1.0),
    }
  }
}

type Config = MeshBufferIntersectConfig;

impl<T> IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config> for Triangle<T>
where
  T: Positioned<Position = Vec3<f32>> + Copy,
{
  #[inline]
  fn intersect(&self, ray: &Ray3, _: &Config) -> OptionalNearest<HitPoint3D> {
    ray.intersect(self, &FaceSide::Double)
  }
}

impl<T> IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config> for LineSegment<T>
where
  T: Positioned<Position = Vec3<f32>> + Copy,
{
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> OptionalNearest<HitPoint3D> {
    let local_tolerance_adjusted =
      conf.line_tolerance.value / conf.current_item_scale_estimate.get();
    ray.intersect(self, &local_tolerance_adjusted)
  }
}

impl<T> IntersectAble<Ray3, OptionalNearest<HitPoint3D>, Config> for Point<T>
where
  T: Positioned<Position = Vec3<f32>> + Copy,
{
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> OptionalNearest<HitPoint3D> {
    ray.intersect(self, &conf.point_tolerance.value)
  }
}
