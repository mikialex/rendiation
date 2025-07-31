use std::time::Instant;

use rendiation_mesh_core::*;
use rendiation_mesh_segmentation::*;
use rendiation_mesh_test_util::for_each_test_mesh_in_folder;

fn main() {
  let config = ClusteringConfig {
    max_vertices: 64,
    max_triangles: 124, // NVidia-recommended 126, rounded down to a multiple of 4
    cone_weight: 0.0,
  };

  // snippet for single mesh debug
  // let path = "/Users/mikialex/dev/resources/obj/bunny.obj";
  // test_segmentation(path, config);

  let obj_test_root = "/Users/mikialex/dev/resources/obj";

  println!("## Start segmentation test in folder:<{:?}>", obj_test_root);
  println!("test config: {:#?}", config);

  println!();
  for_each_test_mesh_in_folder(obj_test_root, |mesh, path| {
    test_segmentation(mesh, path, config);
    println!();
  })
}

fn test_segmentation(mesh: CommonMeshBuffer, obj_path: &str, config: ClusteringConfig) {
  println!("# For input mesh path:<{:?}>:", obj_path);
  println!("  input: face_count: {}", mesh.indices.len() / 3,);

  let start = Instant::now();

  let max_meshlets = build_meshlets_bound(mesh.indices.len(), &config);
  let mut meshlets = vec![rendiation_mesh_segmentation::Meshlet::default(); max_meshlets];

  let mut meshlet_vertices = vec![0; max_meshlets * config.max_vertices as usize];
  let mut meshlet_triangles = vec![0; max_meshlets * config.max_triangles as usize * 3];

  let count = build_meshlets::<_, rendiation_mesh_segmentation::BVHSpaceSearchAcceleration>(
    &config,
    &mesh.indices,
    &mesh.vertices,
    &mut meshlets,
    &mut meshlet_vertices,
    &mut meshlet_triangles,
  );

  let duration = start.elapsed();

  println!(
    "  segmentation result: meshlet_count: {count},  time: {}",
    duration.as_micros() as f64 / 1000.0
  );
}
