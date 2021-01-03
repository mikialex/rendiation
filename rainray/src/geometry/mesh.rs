use rendiation_math::{Vec2, Vec3};
use rendiation_math_entity::{Box3, IntersectAble, Ray3, Triangle};
use rendiation_mesh_buffer::{
  geometry::{AnyGeometry, IndexedGeometry, MeshBufferIntersectConfig, TriangleList},
  vertex::Vertex,
};
use space_indexer::{
  bvh::{FlattenBVH, SAH},
  utils::TreeBuildOption,
};

use crate::{PossibleIntersection, RainRayGeometry};

pub struct Mesh {
  geometry: Box<dyn AnyGeometry<Primitive = Triangle<Vertex>> + Send + Sync>,
  bvh: FlattenBVH<Box3>,
}

impl IntersectAble<Ray3, PossibleIntersection> for Mesh {
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    todo!()
    // self.geometry.as_ref_container().intersect_first_bvh(
    //   ray,
    //   &self.bvh,
    //   MeshBufferIntersectConfig::default(),
    // )
    // let result: NearestPoint3D = ray.intersect(self, param);
    // PossibleIntersection(result.0.map(|near| Intersection {
    //   distance: near.distance,
    //   hit_position: near.position,
    //   hit_normal: self.normal,
    // }))
  }
}
impl RainRayGeometry for Mesh {}

impl Mesh {
  pub fn from_path_obj(path: &str) -> Self {
    let obj = tobj::load_obj("cornell_box.obj", true);
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
            .map(|i| Vertex {
              position: Vec3::new(
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
              ),
              normal: Vec3::new(
                mesh.normals[i * 3],
                mesh.normals[i * 3 + 1],
                mesh.normals[i * 3 + 2],
              ),
              uv: Vec2::new(mesh.texcoords[i * 3], mesh.texcoords[i * 3 + 1]),
            }),
        );
        next_face = end;
      }
    }

    let geometry: IndexedGeometry<_, _, TriangleList> = IndexedGeometry::new(vertices, indices);
    let mut sah = SAH::new(4);
    let bvh = geometry.as_ref_container().build_bvh(
      &mut sah,
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
