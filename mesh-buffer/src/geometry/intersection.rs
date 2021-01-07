use std::cell::Cell;

use super::AnyGeometry;
use rendiation_math_entity::*;

pub trait IntersectAbleAnyGeometry {
  fn intersect_list(&self, ray: Ray3, conf: &Config, result: &mut MeshBufferHitList);
  fn intersect_nearest(&self, ray: Ray3, conf: &Config) -> Nearest<MeshBufferHitPoint>;
}

pub struct MeshBufferHitPoint {
  pub hit: HitPoint3D,
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

impl<G> IntersectAbleAnyGeometry for G
where
  G: AnyGeometry,
  G::Primitive: IntersectAble<Ray3, Nearest<HitPoint3D>, Config>,
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
  fn intersect_nearest(&self, ray: Ray3, conf: &Config) -> Nearest<MeshBufferHitPoint> {
    let mut nearest = Nearest::none();
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

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Copy, Clone)]
pub enum ToleranceType {
  LocalSpace,
  ScreenSpace,
}

pub struct IntersectTolerance {
  pub value: f32,
  pub ty: ToleranceType,
}
impl IntersectTolerance {
  pub fn new(value: f32, ty: ToleranceType) -> Self {
    Self { value, ty }
  }
}

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

impl<T: Positioned<f32, 3>> IntersectAble<Ray3, Nearest<HitPoint3D>, Config> for Triangle<T> {
  #[inline]
  fn intersect(&self, ray: &Ray3, _: &Config) -> Nearest<HitPoint3D> {
    ray.intersect(self, &FaceSide::Double)
  }
}

impl<T: Positioned<f32, 3>> IntersectAble<Ray3, Nearest<HitPoint3D>, Config> for LineSegment<T> {
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> Nearest<HitPoint3D> {
    let local_tolerance_adjusted =
      conf.line_tolerance.value / conf.current_item_scale_estimate.get();
    ray.intersect(self, &local_tolerance_adjusted)
  }
}

impl<T: Positioned<f32, 3>> IntersectAble<Ray3, Nearest<HitPoint3D>, Config> for Point<T> {
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> Nearest<HitPoint3D> {
    ray.intersect(self, &conf.point_tolerance.value)
  }
}

#[test]
fn test() {
  use crate::geometry::*;
  use crate::tessellation::{IndexedGeometryTessellator, Quad};
  use rendiation_math::*;

  let config = MeshBufferIntersectConfig::default();
  let quad = Quad.tessellate();
  let ray = Ray3::new(Vec3::zero(), Vec3::new(1.0, 0.0, 0.0).into_normalized());
  let mut result = MeshBufferHitList::new();
  quad.geometry.intersect_list(ray, &config, &mut result);
}
