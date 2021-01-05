use rendiation_math::{Vec2, Vec3};
use rendiation_math_entity::{Box3, IntersectAble, Ray3, Triangle};
use rendiation_mesh_buffer::{
  geometry::{
    AnyGeometry, BVHIntersectAbleExtendedAnyGeometry, IndexedGeometry, MeshBufferIntersectConfig,
    NoneIndexedGeometry, TriangleList,
  },
  vertex::Vertex,
};
use space_indexer::{
  bvh::{BalanceTree, FlattenBVH, SAH},
  utils::TreeBuildOption,
};

use crate::{Intersection, NormalizedVec3, PossibleIntersection, RainRayGeometry};

pub trait RainrayMeshBuffer: BVHIntersectAbleExtendedAnyGeometry<Box3> + Send + Sync {
  fn get_intersect(&self, ray: &Ray3, bvh: &FlattenBVH<Box3>) -> PossibleIntersection;
}

pub trait TriangleMeshBuffer {
  fn recompute_vertex_normal(&mut self);
}

pub trait HitNormalProvider {
  fn get_normal(&self, point: Vec3<f32>) -> (NormalizedVec3, NormalizedVec3);
}

impl HitNormalProvider for Triangle<Vertex> {
  fn get_normal(&self, _point: Vec3<f32>) -> (NormalizedVec3, NormalizedVec3) {
    let normal = self.face_normal_by_position(); // todo consider cache face normal
    (normal, normal)
  }
}

impl<T> RainrayMeshBuffer for T
where
  T: BVHIntersectAbleExtendedAnyGeometry<Box3> + Send + Sync,
  T: AnyGeometry,
  T::Primitive: HitNormalProvider,
{
  fn get_intersect(&self, ray: &Ray3, bvh: &FlattenBVH<Box3>) -> PossibleIntersection {
    let nearest = self.intersect_first_bvh(*ray, bvh, &MeshBufferIntersectConfig::default());

    PossibleIntersection(nearest.0.map(|hit| {
      let primitive = self.primitive_at(hit.primitive_index);
      let (geometric_normal, shading_normal) = primitive.get_normal(hit.hit.position);
      Intersection {
        distance: hit.hit.distance,
        position: hit.hit.position,
        geometric_normal,
        shading_normal,
      }
    }))
  }
}

pub struct Mesh {
  geometry: Box<dyn RainrayMeshBuffer>,
  bvh: FlattenBVH<Box3>,
}

impl IntersectAble<Ray3, PossibleIntersection> for Mesh {
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    self.geometry.get_intersect(ray, &self.bvh)
  }
}
impl RainRayGeometry for Mesh {}

impl Mesh {
  pub fn from_path_obj(path: &str) -> Self {
    let obj = tobj::load_obj(path, true);
    let (models, _) = obj.unwrap();

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices = Vec::new();

    // we simply merge all groups in obj into one mesh
    for (i, m) in models.iter().enumerate() {
      let mesh = &m.mesh;

      let mut next_face = 0;
      for f in 0..mesh.num_face_indices.len() {
        let count = mesh.num_face_indices[f];
        assert_eq!(count, 3, "obj face should be triangulated");
        let end = next_face + count as usize;
        indices.extend(mesh.indices[next_face..end].iter());
        vertices.extend(
          mesh.indices[next_face..end]
            .iter()
            .map(|&i| i as usize)
            .map(|i| {
              let normal = if mesh.normals.is_empty() {
                Vec3::new(1.0, 0.0, 0.0)
              } else {
                Vec3::new(
                  mesh.normals[i * 3],
                  mesh.normals[i * 3 + 1],
                  mesh.normals[i * 3 + 2],
                )
              };

              let uv = if mesh.texcoords.is_empty() {
                Vec2::new(0.0, 0.0)
              } else {
                Vec2::new(mesh.texcoords[i * 3], mesh.texcoords[i * 3 + 1])
              };

              Vertex {
                position: Vec3::new(
                  mesh.positions[i * 3],
                  mesh.positions[i * 3 + 1],
                  mesh.positions[i * 3 + 2],
                ) * 50.,
                normal,
                uv,
              }
            }),
        );
        next_face = end;
      }
    }

    let geometry: NoneIndexedGeometry<_, TriangleList> = NoneIndexedGeometry::new(vertices);
    use rendiation_mesh_buffer::geometry::BVHExtendedBuildAnyGeometry;
    let bvh = geometry.build_bvh(
      // &mut BalanceTree,
      &mut SAH::new(4),
      &TreeBuildOption {
        max_tree_depth: 50,
        bin_size: 1,
      },
    );
    Mesh {
      geometry: Box::new(geometry),
      bvh,
    }
  }
}
