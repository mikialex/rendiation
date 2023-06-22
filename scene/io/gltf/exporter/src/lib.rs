use std::{collections::HashMap, path::Path};

use gltf_json::Root;
use rendiation_scene_core::*;

pub fn export_scene_to_gltf(scene: &Scene, path: &Path) {
  let scene = scene.read();

  let mut node_mapping = HashMap::<usize, usize>::new();
  let mut nodes = Vec::default();

  // scene.nodes.

  for (_, model) in &scene.models {
    let model = model.read();
    match model.model {
      ModelType::Standard(_) => todo!(),
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
    nodes,
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
    nodes: todo!(),
    samplers: todo!(),
    scenes: todo!(),
    skins: todo!(),
    textures: todo!(),
  };
}
