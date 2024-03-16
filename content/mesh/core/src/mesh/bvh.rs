use rendiation_algebra::Vec3;
use rendiation_geometry::*;
use space_algorithm::{bvh::*, utils::TreeBuildOption};

use super::{
  AbstractMesh, LineList, MeshBufferHitList, MeshBufferHitPoint, MeshBufferIntersectConfig,
  NoneIndexedMesh,
};

pub trait BVHExtendedBuildAbstractMesh<B: BVHBounding, S: BVHBuildStrategy<B>> {
  fn build_bvh(&self, strategy: &mut S, option: &TreeBuildOption) -> FlattenBVH<B>;
}

impl<G, B, S> BVHExtendedBuildAbstractMesh<B, S> for G
where
  B: BVHBounding,
  S: BVHBuildStrategy<B>,
  G: AbstractMesh,
  G::Primitive: SpaceBounding<f32, B, 3>,
{
  fn build_bvh(&self, strategy: &mut S, option: &TreeBuildOption) -> FlattenBVH<B> {
    FlattenBVH::new(
      self.primitive_iter().map(|p| p.to_bounding()),
      strategy,
      option,
    )
  }
}

pub struct PrimitiveIntersectionStatistic {
  pub bound: usize,
  pub primitive: usize,
}

pub trait BVHIntersectAbleExtendedAbstractMesh<B>
where
  B: BVHBounding + IntersectAble<Ray3, bool, ()>,
{
  fn intersect_list_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> MeshBufferHitList;

  fn intersect_nearest_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<MeshBufferHitPoint>;

  fn intersect_nearest_bvh_statistic(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
  ) -> PrimitiveIntersectionStatistic;
}

impl<G, B> BVHIntersectAbleExtendedAbstractMesh<B> for G
where
  B: BVHBounding + IntersectAble<Ray3, bool, ()>,
  G: AbstractMesh,
  G::Primitive: SpaceBounding<f32, B, 3>,
  G::Primitive: IntersectAble<Ray3, OptionalNearest<HitPoint3D>, MeshBufferIntersectConfig>,
{
  fn intersect_list_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> MeshBufferHitList {
    let mut result = MeshBufferHitList::new();
    bvh.traverse(
      |branch| branch.bounding.intersect(&ray, &()),
      |leaf| {
        leaf
          .iter_primitive(bvh)
          .filter_map(|&i| (self.primitive_at(i)?, i).into())
          .filter_map(|(p, primitive_index)| {
            p.intersect(&ray, conf)
              .map(|hit| MeshBufferHitPoint {
                hit,
                primitive_index,
              })
              .0
          })
          .for_each(|h| result.0.push(h));
        true
      },
    );
    result
  }

  fn intersect_nearest_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    bvh.traverse(
      |branch| branch.bounding.intersect(&ray, &()),
      |leaf| {
        leaf
          .iter_primitive(bvh)
          .filter_map(|&i| (self.primitive_at(i)?, i).into())
          .for_each(|(p, primitive_index)| {
            nearest.refresh_nearest(p.intersect(&ray, conf).map(|hit| MeshBufferHitPoint {
              hit,
              primitive_index,
            }));
          });
        true
      },
    );
    nearest
  }

  fn intersect_nearest_bvh_statistic(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
  ) -> PrimitiveIntersectionStatistic {
    let mut bound = 0;
    let mut primitive = 0;
    bvh.traverse(
      |branch| {
        bound += 1;
        branch.bounding.intersect(&ray, &())
      },
      |leaf| {
        primitive += leaf.iter_primitive(bvh).count();
        true
      },
    );
    PrimitiveIntersectionStatistic { bound, primitive }
  }
}

pub trait BVHLineBufferDebugAble {
  fn generate_debug_line_buffer(&self) -> NoneIndexedMesh<LineList, Vec<Vec3<f32>>>;
}

pub trait EntityLineDebugAble {
  fn for_each_line(&self, visitor: &mut impl FnMut(LineSegment<Vec3<f32>>));
}

impl EntityLineDebugAble for Box3 {
  fn for_each_line(&self, visitor: &mut impl FnMut(LineSegment<Vec3<f32>>)) {
    let p0 = Vec3::new(self.min.x, self.min.y, self.min.z); // 000`
    let p1 = Vec3::new(self.min.x, self.min.y, self.max.z); // 001
    let p2 = Vec3::new(self.min.x, self.max.y, self.min.z); // 010
    let p3 = Vec3::new(self.min.x, self.max.y, self.max.z); // 011
    let p4 = Vec3::new(self.max.x, self.min.y, self.min.z); // 100
    let p5 = Vec3::new(self.max.x, self.min.y, self.max.z); // 101
    let p6 = Vec3::new(self.max.x, self.max.y, self.min.z); // 110
    let p7 = Vec3::new(self.max.x, self.max.y, self.max.z); // 111

    let mut line = |a, b| visitor(LineSegment::new(a, b));
    let mut quad = |a, b, c, d| {
      line(a, b);
      line(b, c);
      line(c, d);
      line(d, a);
    };
    quad(p0, p2, p6, p4);
    quad(p1, p3, p7, p5);
    line(p1, p0);
    line(p5, p4);
    line(p7, p6);
    line(p3, p2);
  }
}

impl<B: BVHBounding + EntityLineDebugAble> BVHLineBufferDebugAble for FlattenBVH<B> {
  fn generate_debug_line_buffer(&self) -> NoneIndexedMesh<LineList, Vec<Vec3<f32>>> {
    let mut position = Vec::new();
    self.nodes.iter().for_each(|b| {
      b.bounding.for_each_line(&mut |line| {
        position.push(line.start);
        position.push(line.end);
      })
    });
    NoneIndexedMesh::new(position)
  }
}
