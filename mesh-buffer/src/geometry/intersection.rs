use std::cell::Cell;

use super::{AnyGeometry, AnyGeometryRefContainer};
use rendiation_math_entity::*;

pub trait IntersectableAnyGeometry {
  fn intersect_list(&self, ray: Ray3, conf: &Config, result: &mut IntersectionList3D);
  fn intersect_nearest(&self, ray: Ray3, conf: &Config) -> NearestPoint3D;
}

impl<'a, G> IntersectableAnyGeometry for AnyGeometryRefContainer<'a, G>
where
  G: AnyGeometry,
  G::Primitive: IntersectAble<Ray3, NearestPoint3D, Config>,
{
  fn intersect_list(&self, ray: Ray3, conf: &Config, result: &mut IntersectionList3D) {
    self
      .primitive_iter()
      .into_iter()
      .filter_map(|p| p.intersect(&ray, conf).0)
      .for_each(|h| result.0.push(h))
  }
  fn intersect_nearest(&self, ray: Ray3, conf: &Config) -> NearestPoint3D {
    let mut closest: Option<HitPoint3D> = None;
    self.primitive_iter().into_iter().for_each(|p| {
      let hit = p.intersect(&ray, conf);
      if let NearestPoint3D(Some(h)) = hit {
        if let Some(clo) = &closest {
          if h.distance < clo.distance {
            closest = Some(h)
          }
        } else {
          closest = Some(h)
        }
      }
    });
    NearestPoint3D(closest)
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

impl<T: Positioned<f32, 3>> IntersectAble<Ray3, NearestPoint3D, Config> for Triangle<T> {
  #[inline]
  fn intersect(&self, ray: &Ray3, _: &Config) -> NearestPoint3D {
    ray.intersect(self, &())
  }
}

impl<T: Positioned<f32, 3>> IntersectAble<Ray3, NearestPoint3D, Config> for LineSegment<T> {
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> NearestPoint3D {
    let local_tolerance_adjusted =
      conf.line_tolerance.value / conf.current_item_scale_estimate.get();
    ray.intersect(self, &local_tolerance_adjusted)
  }
}

impl<T: Positioned<f32, 3>> IntersectAble<Ray3, NearestPoint3D, Config> for Point<T> {
  #[inline]
  fn intersect(&self, ray: &Ray3, conf: &Config) -> NearestPoint3D {
    ray.intersect(self, &conf.point_tolerance.value)
  }
}

#[test]
fn test() {
  use super::{IndexedGeometry, TriangleList};
  use crate::geometry::container::AnyGeometry;
  use crate::tessellation::{IndexedBufferTessellator, Quad};
  use rendiation_math::*;

  let config = MeshBufferIntersectConfig::default();
  let quad = Quad.create_mesh(&());
  let quad = IndexedGeometry::<u16, _, TriangleList>::from(quad);
  let ray = Ray::new(Vec3::zero(), Vec3::new(1.0, 0.0, 0.0));
  let mut result = IntersectionList3D::new();
  quad
    .as_ref_container()
    .intersect_list(ray, &config, &mut result);
}
