use std::path::Path;

use gltf::{Node, Result};
use rendiation_algebra::*;
use rendiation_scene_webgpu::{
  MeshModel, MeshModelImpl, PhysicalMaterial, Scene, SceneNode, StateControl, WebGPUScene,
};

pub fn load_gltf_test(path: impl AsRef<Path>, scene: &mut Scene<WebGPUScene>) -> Result<()> {
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
  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(scene.root(), &node);
    }
  }
  Ok(())
}

/// https://docs.rs/gltf/latest/gltf/struct.Node.html
fn create_node_recursive(parent_to_attach: &SceneNode, gltf_node: &Node) {
  println!(
    "Node #{} has {} children",
    gltf_node.index(),
    gltf_node.children().count(),
  );

  let node = parent_to_attach.create_child();

  let local_mat = match gltf_node.transform() {
    gltf::scene::Transform::Matrix { matrix } => {
      Mat4::new_from_colum(matrix[0], matrix[1], matrix[2], matrix[3])
    }
    gltf::scene::Transform::Decomposed {
      translation,
      rotation,
      scale,
    } => {
      Mat4::scale(scale)
        * Mat4::from(Quat::new(
          rotation[0],
          rotation[1],
          rotation[2],
          rotation[3],
        ))
        * Mat4::translate(translation)
    }
  };
  node.set_local_matrix(local_mat);

  if let Some(mesh) = gltf_node.mesh() {
    for primitive in mesh.primitives() {
      let material = build_pbr_material(primitive.material());
      //
      // let model: MeshModel<_, _> = MeshModelImpl::new(material, mesh, node).into();
    }
  }

  for gltf_node in gltf_node.children() {
    create_node_recursive(&node, &gltf_node)
  }
}

/// https://docs.rs/gltf/latest/gltf/struct.Material.html
fn build_pbr_material(material: gltf::Material) -> StateControl<PhysicalMaterial<WebGPUScene>> {
  todo!()
}
