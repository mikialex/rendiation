use crate::*;

pub trait AbstractMeshIntersectionExt<C> {
  fn ray_intersect_iter(&self, ray: Ray3, conf: &C) -> impl Iterator<Item = MeshBufferHitPoint>;

  fn ray_intersect_all(&self, ray: Ray3, conf: &C, result: &mut Vec<MeshBufferHitPoint>) {
    self
      .ray_intersect_iter(ray, conf)
      .for_each(|h| result.push(h));
  }
  fn ray_intersect_nearest(&self, ray: Ray3, conf: &C) -> OptionalNearest<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    self.ray_intersect_iter(ray, conf).for_each(|r| {
      nearest.refresh_nearest(OptionalNearest::some(r));
    });
    nearest
  }
}

impl<C, G> AbstractMeshIntersectionExt<C> for G
where
  G: AbstractMesh,
  Ray3: IntersectAble<G::Primitive, OptionalNearest<HitPoint3D>, C>,
{
  fn ray_intersect_iter(&self, ray: Ray3, conf: &C) -> impl Iterator<Item = MeshBufferHitPoint> {
    self
      .primitive_iter()
      .enumerate()
      .filter_map(move |(primitive_index, p)| {
        ray
          .intersect(&p, conf)
          .map(|hit| MeshBufferHitPoint {
            hit,
            primitive_index,
          })
          .0
      })
  }
}

#[derive(Copy, Clone, Debug)]
pub struct MeshBufferHitPoint<T: Scalar = f32> {
  pub hit: HitPoint3D<T>,
  pub primitive_index: usize,
}
impl HitDistanceCompareAble for MeshBufferHitPoint {
  fn is_near_than(&self, other: &Self) -> bool {
    self.hit.is_near_than(&other.hit)
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
