use std::path::Path;
use std::{fmt::Debug, time::Instant};

use rendiation_algebra::{Vec2, Vec3};
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_simplification::*;

fn main() {
  let mesh = Mesh::load("/Users/mikialex/dev/resources/testdata/bunny.obj").unwrap();
  let mut dest_idx = mesh.indices.clone();

  let start = Instant::now();

  let EdgeCollapseResult {
    result_error,
    result_count,
  } = simplify_by_edge_collapse(
    &mut dest_idx,
    &mesh.indices,
    &mesh.vertices,
    None,
    EdgeCollapseConfig {
      target_index_count: 500,
      target_error: 100.,
      lock_border: false,
    },
  );

  let duration = start.elapsed();

  println!(
    "result count: {result_count}, error: {result_error}, time: {}",
    duration.as_micros() as f64 / 1000.0
  );
}

#[derive(Clone, Default)]
struct Mesh {
  vertices: Vec<CommonVertex>,
  indices: Vec<u32>,
}

impl Mesh {
  pub fn load<P>(path: P) -> Result<Mesh, tobj::LoadError>
  where
    P: AsRef<Path> + Clone + Debug,
  {
    let (models, _materials) = tobj::load_obj(
      path.clone(),
      &tobj::LoadOptions {
        triangulate: true,
        ..Default::default()
      },
    )?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for model in &models {
      let mesh = &model.mesh;
      assert!(mesh.positions.len() % 3 == 0);

      vertices.reserve(mesh.indices.len());
      indices.extend_from_slice(&mesh.indices);

      for i in 0..mesh.indices.len() {
        let pi = mesh.indices[i] as usize;
        let ni = mesh.normal_indices[i] as usize;
        let ti = mesh.texcoord_indices[i] as usize;

        let position: [f32; 3] = mesh.positions[3 * pi..3 * (pi + 1)].try_into().unwrap();
        let position = position.into();

        let normal = if !mesh.normals.is_empty() {
          let normal: [f32; 3] = mesh.normals[3 * ni..3 * (ni + 1)].try_into().unwrap();
          normal.into()
        } else {
          Vec3::new(1., 0., 0.)
        };

        let uv = if !mesh.texcoords.is_empty() {
          let uv: [f32; 2] = mesh.texcoords[2 * ti..2 * (ti + 1)].try_into().unwrap();
          uv.into()
        } else {
          Vec2::new(0., 0.)
        };

        vertices.push(CommonVertex {
          position,
          normal,
          uv,
        });
      }
    }

    let total_indices = indices.len();
    let mut remap = vec![0; total_indices];

    let mut result = Mesh::default();

    let total_vertices = generate_vertex_remap(&mut remap, None, &vertices);

    result.indices = remap;

    result
      .vertices
      .resize(total_vertices, CommonVertex::default());
    remap_vertex_buffer(&mut result.vertices, &vertices, &result.indices);

    println!(
      "# {:?}: {} vertices, {} triangles",
      path,
      result.vertices.len(),
      total_indices / 3,
    );

    Ok(result)
  }
}
