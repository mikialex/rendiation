// use std::cmp::Ordering;

use rendiation_algebra::{Vec2, Vec3};
use rendiation_geometry::{Box3, IntersectAble, Ray3, Triangle};
use rendiation_renderable_mesh::{
  geometry::{
    AnyGeometry, BVHIntersectAbleExtendedAnyGeometry, MeshBufferIntersectConfig,
    NoneIndexedGeometry, TriangleList,
  },
  vertex::Vertex,
};
use space_algorithm::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};

use crate::{Intersection, NormalizedVec3, PossibleIntersection, RainRayGeometry, Scene};

pub trait RainrayMeshBuffer: Send + Sync {
  fn get_intersect(&self, ray: &Ray3) -> PossibleIntersection;
}

pub trait ShadingNormalProvider {
  fn get_normal(&self, point: Vec3<f32>) -> NormalizedVec3;
}

impl ShadingNormalProvider for Triangle<Vertex> {
  fn get_normal(&self, point: Vec3<f32>) -> NormalizedVec3 {
    let barycentric = self
      .map_position()
      .barycentric(point)
      .unwrap_or(Vec3::new(1., 0., 0.));
    let normal =
      barycentric.x * self.a.normal + barycentric.y * self.b.normal + barycentric.z * self.c.normal;
    use rendiation_algebra::IntoNormalizedVector;
    unsafe { normal.into_normalized_unchecked() }
  }
}

pub struct TriangleMesh<G> {
  pub geometry: G,
  pub face_normal: Vec<NormalizedVec3>,
  pub bvh: FlattenBVH<Box3>,
}

impl<G> TriangleMesh<G>
where
  G: AnyGeometry<Primitive = Triangle<Vertex>>,
  G: BVHIntersectAbleExtendedAnyGeometry<Box3>,
{
  pub fn new(geometry: G) -> Self {
    use rendiation_renderable_mesh::geometry::BVHExtendedBuildAnyGeometry;
    let bvh = geometry.build_bvh(
      &mut SAH::new(4),
      &TreeBuildOption {
        max_tree_depth: 50,
        bin_size: 1,
      },
    );
    let face_normal = geometry
      .primitive_iter()
      .map(|p| p.map_position().face_normal())
      .collect();
    Self {
      geometry,
      face_normal,
      bvh,
    }
  }
  pub fn recompute_vertex_normal(&mut self) {
    // need impl mut_primitive_iter
    // self.geometry.primitive_iter()
  }
}

impl<G> RainrayMeshBuffer for TriangleMesh<G>
where
  G: BVHIntersectAbleExtendedAnyGeometry<Box3> + Send + Sync,
  G: AnyGeometry,
  G::Primitive: ShadingNormalProvider,
{
  fn get_intersect(&self, ray: &Ray3) -> PossibleIntersection {
    let nearest =
      self
        .geometry
        .intersect_first_bvh(*ray, &self.bvh, &MeshBufferIntersectConfig::default());

    PossibleIntersection(nearest.0.map(|hit| {
      let primitive = self.geometry.primitive_at(hit.primitive_index);
      // let geometric_normal = self.face_normal[hit.primitive_index];
      let shading_normal = primitive.get_normal(hit.hit.position);
      Intersection {
        distance: hit.hit.distance,
        position: hit.hit.position,
        geometric_normal: shading_normal,
        shading_normal,
      }
    }))
  }
}

pub struct Mesh {
  geometry: Box<dyn RainrayMeshBuffer>,
}

impl IntersectAble<Ray3, PossibleIntersection, Scene> for Mesh {
  fn intersect(&self, ray: &Ray3, _param: &Scene) -> PossibleIntersection {
    self.geometry.get_intersect(ray)
  }
}
impl RainRayGeometry for Mesh {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
}

impl Mesh {
  pub fn from_path_obj(path: &str) -> Self {
    let obj = tobj::load_obj(path, true);
    let (models, _) = obj.unwrap();

    let mut indices: Vec<u32> = Vec::new();
    let mut vertices = Vec::new();
    let mut need_compute_vertex_normal = false;

    // we simply merge all groups in obj into one mesh
    for (_i, m) in models.iter().enumerate() {
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

    let mut geometry: NoneIndexedGeometry<_, TriangleList> = NoneIndexedGeometry::new(vertices);

    if need_compute_vertex_normal {
      let face_normals: Vec<NormalizedVec3> = geometry
        .primitive_iter()
        .map(|p| p.map_position().face_normal())
        .collect();

      use rendiation_algebra::Vector;
      geometry
        .data
        .iter_mut()
        .for_each(|v| v.normal = Vec3::zero());

      for i in 0..geometry.data.len() / 3 {
        for j in 0..3 {
          let v = &mut geometry.data[i * 3 + j];
          v.normal = v.normal + face_normals[i].value
        }
      }
      use rendiation_algebra::InnerProductSpace;
      geometry
        .data
        .iter_mut()
        .for_each(|v| v.normal = v.normal.normalize());
    }

    let geometry = geometry.create_index_geometry();
    // let geometry = geometry.merge_vertex_by_sorting(
    //   |a, b| {
    //     a.position
    //       .x
    //       .partial_cmp(&b.position.x)
    //       .unwrap_or(Ordering::Equal)
    //   },
    //   |a, b| a.position.x - b.position.y <= 0.0001,
    // );

    let mesh = TriangleMesh::new(geometry);
    Mesh {
      geometry: Box::new(mesh),
    }
  }
}
