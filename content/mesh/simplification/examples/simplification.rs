use std::ffi::OsStr;
use std::path::Path;
use std::{fmt::Debug, time::Instant};

use rendiation_algebra::{Vec2, Vec3};
use rendiation_mesh_core::{create_deduplicated_index_vertex_mesh, CommonVertex};
use rendiation_mesh_simplification::*;
use walkdir::WalkDir;

fn main() {
  let config = EdgeCollapseConfig {
    target_index_count: 500,
    target_error: 100.,
    lock_border: false,
    use_absolute_error: false,
  };

  // snippet for single mesh debug
  // let path = "/Users/mikialex/dev/resources/obj/bunny.obj";
  // test_simplification(path, config);

  let obj_test_root = "/Users/mikialex/dev/resources/obj";

  println!(
    "## Start simplification test in folder:<{:?}>",
    obj_test_root
  );
  println!("test config: {:#?}", config);

  println!();
  for entry in WalkDir::new(obj_test_root).into_iter() {
    let entry = entry.unwrap();

    if entry.path().is_dir() {
      continue;
    }

    if let Some(extension) = entry.path().extension().and_then(OsStr::to_str) {
      if extension == "obj" {
        test_simplification(entry.path(), config);
        println!();
      }
    }
  }
}

fn test_simplification(obj_path: impl AsRef<Path> + Clone + Debug, config: EdgeCollapseConfig) {
  let mesh = match Mesh::load(obj_path.clone()) {
    Ok(mesh) => mesh,
    Err(err) => {
      println!("# Obj parse error:<{:?}>: {}", obj_path, err);
      return;
    }
  };

  if config.target_index_count > mesh.indices.len() {
    println!(
      "# Input mesh is too simple for simplification:<{:?}>:",
      obj_path
    );
    return;
  }

  println!("# For input mesh path:<{:?}>:", obj_path);
  println!("  input: face_count: {}", mesh.indices.len() / 3,);

  let mut dest_idx = mesh.indices.clone();

  let start = Instant::now();

  let SimplificationResult {
    result_error,
    result_count,
  } = simplify_by_edge_collapse(&mut dest_idx, &mesh.indices, &mesh.vertices, None, config);

  let duration = start.elapsed();

  let result_face_count = result_count / 3;

  println!(
    "  simplified result: face_count: {result_face_count}, error: {result_error}, time: {}",
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

    let (indices, vertices) =
      create_deduplicated_index_vertex_mesh(indices.iter().map(|i| vertices[*i as usize]));

    Ok(Mesh { vertices, indices })
  }
}
