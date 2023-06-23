use std::fs;
use std::{collections::HashMap, path::Path};

use gltf_json::Root;
use rendiation_scene_core::*;

// pub fn write_texture(texture: &SceneTexture2D, textures: &Vec<gltf_json::Texture>, map:
// &HashMap<usize, usize>, path: &Path) {   //
// }

// pub fn write_model(model: models: &Vec<gltf_json::Mesh>, map: &HashMap<usize, usize>, path:
// &Path) {   //
// }

pub enum GltfExportErr {
  IO(std::io::Error),
}

pub fn export_scene_to_gltf(scene: &Scene, path: &Path) -> Result<(), GltfExportErr> {
  fs::create_dir_all(path).map_err(GltfExportErr::IO)?;

  let scene = scene.read();

  let mut node_mapping = HashMap::<usize, usize>::new();
  let mut nodes = Vec::<gltf_json::Node>::default();
  let mut scene_node_ids = Vec::default();

  // scene.nodes.

  let mut meshes = Vec::default();
  let mut mesh_mapping = HashMap::<usize, gltf_json::Index<gltf_json::Mesh>>::new();

  for (_, model) in &scene.models {
    let model = model.read();
    let node_idx = *node_mapping.get(&model.node.guid()).unwrap();
    let node = nodes.get_mut(node_idx).unwrap();

    match &model.model {
      ModelType::Standard(model) => {
        node.mesh = Some(*mesh_mapping.entry(model.guid()).or_insert_with(|| {
          let idx = meshes.len();
          meshes.push(gltf_json::Mesh {
            extensions: Default::default(),
            extras: Default::default(),
            name: Default::default(),
            primitives: todo!(),
            weights: todo!(),
          });
          gltf_json::Index::new(idx as u32)
        }));
      }
      _ => todo!(),
    }
  }

  for (_, light) in &scene.lights {
    let light = light.read();
    match light.light {
      SceneLightKind::PointLight(_) => todo!(),
      SceneLightKind::SpotLight(_) => todo!(),
      SceneLightKind::DirectionalLight(_) => todo!(),
      _ => todo!(),
    }
  }

  for (_, camera) in &scene.cameras {
    let camera = camera.read();
    match camera.projection {
      CameraProjector::Perspective(_) => todo!(),
      CameraProjector::ViewOrthographic(_) => todo!(),
      CameraProjector::Orthographic(_) => todo!(),
      _ => todo!(),
    }
  }

  let scene = gltf_json::Scene {
    nodes: scene_node_ids,
    extensions: Default::default(),
    extras: Default::default(),
    name: Default::default(),
  };

  let json = Root {
    accessors: todo!(),
    animations: todo!(),
    asset: todo!(),
    buffers: todo!(),
    buffer_views: todo!(),
    scene: todo!(),
    extensions: todo!(),
    extras: todo!(),
    extensions_used: todo!(),
    extensions_required: todo!(),
    cameras: todo!(),
    images: todo!(),
    materials: todo!(),
    meshes: todo!(),
    nodes,
    samplers: todo!(),
    scenes: todo!(),
    skins: todo!(),
    textures: todo!(),
  };

  Ok(())
}
