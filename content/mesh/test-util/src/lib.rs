use std::fmt::Debug;
use std::{ffi::OsStr, path::Path};

use rendiation_algebra::*;
use rendiation_mesh_core::*;
use walkdir::WalkDir;

pub fn for_each_test_mesh_in_folder(
  folder: impl AsRef<Path>,
  mut f: impl FnMut(CommonMeshBuffer, &str),
) {
  for entry in WalkDir::new(folder).into_iter() {
    let entry = entry.unwrap();

    if entry.path().is_dir() {
      continue;
    }

    if let Some(extension) = entry.path().extension().and_then(OsStr::to_str) {
      if extension == "obj" {
        let mesh = match load_common_mesh(entry.path()) {
          Ok(mesh) => mesh,
          Err(err) => {
            println!("# Obj parse error:<{:?}>: {}", entry.path(), err);
            continue;
          }
        };

        f(mesh, entry.path().to_str().unwrap());
      }
    }
  }
}

pub fn load_common_mesh<P>(path: P) -> Result<CommonMeshBuffer, tobj::LoadError>
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

  Ok(CommonMeshBuffer { vertices, indices })
}
