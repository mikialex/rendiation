use std::path::Path;

use gltf::{Gltf, Result};
use rendiation_scene_webgpu::{Scene, WebGPUScene};

pub fn load_gltf_test(path: impl AsRef<Path>, scene: &Scene<WebGPUScene>) -> Result<()> {
  // let gltf = Gltf::open(path)?;
  // for scene in gltf.scenes() {
  //   for node in scene.nodes() {
  //     println!(
  //       "Node #{} has {} children",
  //       node.index(),
  //       node.children().count(),
  //     );
  //   }
  // }
  let (document, buffers, images) = gltf::import(path)?;
  for scene in document.scenes() {
    for node in scene.nodes() {
      println!(
        "Node #{} has {} children",
        node.index(),
        node.children().count(),
      );
    }
  }
  Ok(())
}
