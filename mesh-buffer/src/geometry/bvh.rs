use super::{AnyGeometry, LineList, MeshBufferIntersectConfig, NoneIndexedGeometry};
use rendiation_math::Vec3;
use rendiation_math_entity::*;
use space_indexer::{bvh::*, utils::TreeBuildOption};

pub trait BVHExtendedAnyGeometry<B, S>
where
  B: BVHBounding + IntersectAble<Ray3, bool, ()>,
  S: BVHBuildStrategy<B>,
{
  fn build_bvh(&self, strategy: &mut S, option: &TreeBuildOption) -> FlattenBVH<B>;

  fn intersect_list_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> IntersectionList3D;

  fn intersect_first_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> Nearest<HitPoint3D>;
}

impl<G, B, S> BVHExtendedAnyGeometry<B, S> for G
where
  B: BVHBounding + IntersectAble<Ray3, bool, ()>,
  S: BVHBuildStrategy<B>,
  G: AnyGeometry,
  G::Primitive: SpaceBounding<f32, B, 3>,
  G::Primitive: IntersectAble<Ray3, Nearest<HitPoint3D>, MeshBufferIntersectConfig>,
{
  fn build_bvh(&self, strategy: &mut S, option: &TreeBuildOption) -> FlattenBVH<B> {
    FlattenBVH::new(
      self.primitive_iter().map(|p| p.to_bounding()),
      strategy,
      option,
    )
  }

  fn intersect_list_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> IntersectionList3D {
    let mut result = IntersectionList3D::new();
    bvh.traverse(
      |branch| branch.bounding.intersect(&ray, &()),
      |leaf| {
        leaf
          .iter_primitive(bvh)
          .map(|&i| self.primitive_at(i))
          .for_each(|p| result.push_nearest(p.intersect(&ray, conf)))
      },
    );
    result
  }

  fn intersect_first_bvh(
    &self,
    ray: Ray3,
    bvh: &FlattenBVH<B>,
    conf: &MeshBufferIntersectConfig,
  ) -> Nearest<HitPoint3D> {
    todo!()
    // let mut result = IntersectionList3D::new();
    // bvh.traverse(
    //   |branch| branch.bounding.intersect(&ray, &()),
    //   |leaf| {
    //     leaf
    //       .iter_primitive(bvh)
    //       .map(|&i| self.geometry.primitive_at(i))
    //       .for_each(|p| result.push_nearest(p.intersect(&ray, conf)))
    //   },
    // );
    // result
  }
}

pub trait BVHLineBufferDebugAble {
  fn generate_debug_line_buffer(&self) -> NoneIndexedGeometry<Vec3<f32>, LineList>;
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
  fn generate_debug_line_buffer(&self) -> NoneIndexedGeometry<Vec3<f32>, LineList> {
    let mut position = Vec::new();
    self.nodes.iter().for_each(|b| {
      b.bounding.for_each_line(&mut |line| {
        position.push(line.start);
        position.push(line.end);
      })
    });
    NoneIndexedGeometry::new(position)
  }
}
