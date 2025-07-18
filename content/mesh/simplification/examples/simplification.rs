use std::ffi::OsStr;
use std::path::Path;
use std::{fmt::Debug, time::Instant};

use rendiation_algebra::{Vec2, Vec3};
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_simplification::*;
use walkdir::WalkDir;

fn main() {
  // let path = "/Users/mikialex/dev/resources/obj/bunny.obj";
  // test_simplification(path);

  let obj_test_root = "/Users/mikialex/dev/resources/obj";

  for entry in WalkDir::new(obj_test_root).into_iter() {
    let entry = entry.unwrap();

    if entry.path().is_dir() {
      continue;
    }

    if let Some(extension) = entry.path().extension().and_then(OsStr::to_str) {
      if extension == "obj" {
        test_simplification(entry.path());
      }
    }
  }
}

fn test_simplification(obj_path: impl AsRef<Path> + Clone + Debug) {
  let mesh = Mesh::load(obj_path.clone()).unwrap();

  println!("# For input mesh path:<{:?}>:", obj_path);
  println!("  input: face_count: {}", mesh.indices.len() / 3,);

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

  let result_face_count = result_count / 3;

  println!(
    "  simplified result: face_count: {result_face_count}, error: {result_error}, time: {}",
    duration.as_micros() as f64 / 1000.0
  );
  println!();
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

        let position: [f32; 3] = mesh.positions[3 * pi..3 * (pi + 1)].try_into().unwrap();
        let position = position.into();

        let normal = if !mesh.normals.is_empty() {
          let ni = mesh.normal_indices[i] as usize;
          let normal: [f32; 3] = mesh.normals[3 * ni..3 * (ni + 1)].try_into().unwrap();
          normal.into()
        } else {
          Vec3::new(1., 0., 0.)
        };

        let uv = if !mesh.texcoords.is_empty() {
          let ti = mesh.texcoord_indices[i] as usize;
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

    result
      .vertices
      .resize(total_vertices, CommonVertex::default());
    let mut indices = vec![0; total_indices];

    remap_vertex_buffer(&mut result.vertices, &vertices, &remap);
    remap_index_buffer(&mut indices, None, total_indices, &remap);
    result.indices = indices;

    Ok(result)
  }
}
