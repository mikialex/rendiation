use std::time::Instant;

use rendiation_mesh_core::*;
use rendiation_mesh_simplification::*;
use rendiation_mesh_test_util::for_each_test_mesh_in_folder;

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
  for_each_test_mesh_in_folder(obj_test_root, |mesh, path| {
    test_simplification(mesh, path, config);
    println!();
  })
}

fn test_simplification(mesh: CommonMeshBuffer, obj_path: &str, config: EdgeCollapseConfig) {
  println!("# For input mesh path:<{:?}>:", obj_path);
  println!("  input: face_count: {}", mesh.indices.len() / 3,);

  {
    let mut dest_idx = mesh.indices.clone();

    let start = Instant::now();

    let SimplificationResult {
      result_error,
      result_count,
    } = simplify_by_edge_collapse(&mut dest_idx, &mesh.indices, &mesh.vertices, None, config);

    let duration = start.elapsed();

    let result_face_count = result_count / 3;

    println!(
      "  edge collapse simplified result: face_count: {result_face_count}, error: {result_error}, time: {}",
      duration.as_micros() as f64 / 1000.0
    );
  }

  {
    let mut dest_idx = mesh.indices.clone();

    let start = Instant::now();

    let SimplificationResult {
      result_error,
      result_count,
    } = simplify_sloppy(
      &mut dest_idx,
      &mesh.indices,
      &mesh.vertices,
      None,
      mesh.indices.len() as u32 / 2,
      f32::INFINITY,
      true,
    );

    let duration = start.elapsed();

    let result_face_count = result_count / 3;

    println!(
      "  sloppy simplified result: face_count: {result_face_count}, error: {result_error}, time: {}",
      duration.as_micros() as f64 / 1000.0
    );
  }
}
