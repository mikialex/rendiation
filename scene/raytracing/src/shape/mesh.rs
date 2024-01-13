use std::sync::Arc;

use rendiation_algebra::{Vec2, Vec3};
use rendiation_geometry::{Box3, Ray3, Triangle};
use rendiation_mesh_core::{vertex::Vertex, *};
use space_algorithm::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};

use crate::*;

pub trait ShadingNormalProvider {
  fn get_normal(&self, point: Vec3<f32>) -> NormalizedVec3<f32>;
}

impl ShadingNormalProvider for Triangle<Vertex> {
  fn get_normal(&self, point: Vec3<f32>) -> NormalizedVec3<f32> {
    let barycentric = self
      .map(|v| v.position())
      .barycentric(point)
      .unwrap_or(Vec3::new(1., 0., 0.));
    let normal =
      barycentric.x * self.a.normal + barycentric.y * self.b.normal + barycentric.z * self.c.normal;
    unsafe { normal.into_normalized_unchecked() }
  }
}

pub struct TriangleMesh<G> {
  pub mesh: G,
  pub face_normal: Vec<NormalizedVec3<f32>>,
  pub bvh: FlattenBVH<Box3>,
}

impl<G> TriangleMesh<G>
where
  G: AbstractMesh<Primitive = Triangle<Vertex>>,
  G: BVHIntersectAbleExtendedAbstractMesh<Box3>,
{
  pub fn new(mesh: G) -> Self {
    let bvh = mesh.build_bvh(
      &mut SAH::new(4),
      &TreeBuildOption {
        max_tree_depth: 50,
        bin_size: 1,
      },
    );
    let face_normal = mesh
      .primitive_iter()
      .map(|p| p.map(|v| v.position()).face_normal())
      .collect();
    Self {
      mesh,
      face_normal,
      bvh,
    }
  }
}

impl<G> Shape for Arc<TriangleMesh<G>>
where
  G: BVHIntersectAbleExtendedAbstractMesh<Box3> + Send + Sync + 'static,
  G: AbstractMesh<Primitive = Triangle<Vertex>>,
{
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn intersect(&self, ray: Ray3) -> Option<Intersection> {
    let nearest =
      self
        .mesh
        .intersect_nearest_bvh(ray, &self.bvh, &MeshBufferIntersectConfig::default());

    nearest.0.map(|hit| {
      let primitive = self.mesh.primitive_at(hit.primitive_index).unwrap();
      let geometric_normal = self.face_normal[hit.primitive_index];
      let shading_normal = primitive.get_normal(hit.hit.position);
      Intersection {
        position: hit.hit.position,
        geometric_normal,
        shading_normal,
        uv: None,
      }
    })
  }

  fn get_bbox(&self) -> Option<Box3> {
    None
  }

  fn intersect_statistic(&self, ray: Ray3) -> IntersectionStatistic {
    let stat = self.mesh.intersect_nearest_bvh_statistic(ray, &self.bvh);
    IntersectionStatistic {
      box3: stat.bound,
      sphere: 0,
      triangle: stat.primitive,
    }
  }
}

impl TriangleMesh<IndexedMesh<TriangleList, Vec<Vertex>, Vec<u32>>> {
  pub fn from_path_obj(path: &str) -> Self {
    let obj = tobj::load_obj(path, true);
    let (models, _) = obj.unwrap();

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices = Vec::new();
    let mut need_compute_vertex_normal = false;

    // we simply merge all groups in obj into one mesh
    for m in &models {
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
                need_compute_vertex_normal = true;
                Vec2::new(0.0, 0.0)
              } else {
                Vec2::new(mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1])
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

    let mut mesh: NoneIndexedMesh<TriangleList, Vec<Vertex>> = NoneIndexedMesh::new(vertices);

    if need_compute_vertex_normal {
      let face_normals: Vec<NormalizedVec3<f32>> = mesh
        .primitive_iter()
        .map(|p| p.map(|v| v.position()).face_normal())
        .collect();

      mesh.data.iter_mut().for_each(|v| v.normal = Vec3::zero());

      #[allow(clippy::needless_range_loop)]
      for i in 0..mesh.data.len() / 3 {
        for j in 0..3 {
          let v = &mut mesh.data[i * 3 + j];
          v.normal += face_normals[i].value
        }
      }
      mesh
        .data
        .iter_mut()
        .for_each(|v| v.normal = v.normal.normalize());
    }

    let mesh: IndexedMesh<TriangleList, Vec<Vertex>, Vec<u32>> = mesh.primitive_iter().collect();
    use std::cmp::Ordering;
    #[allow(clippy::float_cmp)]
    let mesh = mesh
      .merge_vertex_by_sorting(
        |a, b| {
          a.position
            .x
            .partial_cmp(&b.position.x)
            .unwrap_or(Ordering::Equal)
        },
        |a, b| a.position.x == b.position.x,
      )
      .unwrap();

    TriangleMesh::new(mesh)
  }
}
