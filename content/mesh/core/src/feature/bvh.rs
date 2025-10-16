use rendiation_space_algorithm::{bvh::*, utils::TreeBuildOption};

use crate::*;

pub fn build_bvh_for_abstract_mesh<G, B, S>(
  mesh: &G,
  strategy: &mut S,
  option: &TreeBuildOption,
) -> FlattenBVH<B>
where
  B: BVHBounding,
  S: BVHBuildStrategy<B>,
  G: AbstractMesh,
  G::Primitive: SpaceBounding<f32, B, 3>,
{
  FlattenBVH::new(
    mesh.primitive_iter().map(|p| p.to_bounding()),
    strategy,
    option,
  )
}

pub fn intersect_list_bvh<G, B, C>(
  mesh: &G,
  ray: Ray3,
  bvh: &FlattenBVH<B>,
  conf: &C,
) -> MeshBufferHitList
where
  B: BVHBounding + IntersectAble<Ray3, bool, ()>,
  G: AbstractMesh,
  G::Primitive: SpaceBounding<f32, B, 3>,
  G::Primitive: IntersectAble<Ray3, OptionalNearest<HitPoint3D>, C>,
{
  let mut result = MeshBufferHitList::new();
  bvh.traverse_branch_leaf_visitor(
    |branch| branch.bounding.intersect(&ray, &()),
    |leaf| {
      leaf
        .iter_primitive(bvh)
        .filter_map(|&i| (mesh.primitive_at(i)?, i).into())
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

pub fn intersect_nearest_bvh<G, B, C>(
  mesh: &G,
  ray: Ray3,
  bvh: &FlattenBVH<B>,
  conf: &C,
) -> OptionalNearest<MeshBufferHitPoint>
where
  B: BVHBounding + IntersectAble<Ray3, bool, ()>,
  G: AbstractMesh,
  G::Primitive: SpaceBounding<f32, B, 3>,
  G::Primitive: IntersectAble<Ray3, OptionalNearest<HitPoint3D>, C>,
{
  let mut nearest = OptionalNearest::none();
  bvh.traverse_branch_leaf_visitor(
    |branch| branch.bounding.intersect(&ray, &()),
    |leaf| {
      leaf
        .iter_primitive(bvh)
        .filter_map(|&i| (mesh.primitive_at(i)?, i).into())
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

pub trait BVHLineBufferDebugAbleExt {
  fn generate_debug_line_buffer(&self) -> NoneIndexedMesh<LineList, Vec<Vec3<f32>>>;
}

impl<B: BVHBounding + EntityLineDebugAble> BVHLineBufferDebugAbleExt for FlattenBVH<B> {
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
