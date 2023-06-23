use std::fs::{self, File};
use std::io::BufWriter;
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

pub fn export_scene_to_gltf(
  scene: &Scene,
  folder_path: &Path,
  file_name: &str,
) -> Result<(), GltfExportErr> {
  fs::create_dir_all(folder_path).map_err(GltfExportErr::IO)?;

  let scene = scene.read();

  let mut node_mapping = HashMap::<usize, usize>::new();
  let mut nodes = Vec::<gltf_json::Node>::default();
  let mut scene_node_ids = Vec::default();

  // todo load scene.nodes.

  let mut models = Vec::default();
  let mut model_mapping = HashMap::<usize, gltf_json::Index<gltf_json::Mesh>>::new();

  let mut materials = Vec::default();
  let mut material_mapping = HashMap::<usize, gltf_json::Index<gltf_json::Material>>::new();

  for (_, model) in &scene.models {
    let model = model.read();
    let node_idx = *node_mapping.get(&model.node.guid()).unwrap();
    let node = nodes.get_mut(node_idx).unwrap();

    match &model.model {
      ModelType::Standard(model) => {
        node.mesh = Some(*model_mapping.entry(model.guid()).or_insert_with(|| {
          let idx = models.len();

          let model = model.read();
          match &model.mesh {
            SceneMeshType::AttributesMesh(mesh) => {
              let mesh = mesh.read();
              //
            }
            SceneMeshType::TransformInstanced(_) => todo!(),
            SceneMeshType::Foreign(_) => todo!(),
            _ => todo!(),
          }

          match &model.material {
            SceneMaterialType::PhysicalSpecularGlossiness(material) => {
              material_mapping.entry(material.guid()).or_insert_with(|| {
                let idx = materials.len();

                let material = material.read();
                materials.push(gltf_json::Material {
                  alpha_cutoff: todo!(),
                  alpha_mode: todo!(),
                  double_sided: todo!(),
                  name: todo!(),
                  pbr_metallic_roughness: todo!(),
                  normal_texture: todo!(),
                  occlusion_texture: todo!(),
                  emissive_texture: todo!(),
                  emissive_factor: todo!(),
                  extensions: Default::default(),
                  extras: Default::default(),
                });
                gltf_json::Index::new(idx as u32)
              });
              //
            }
            SceneMaterialType::PhysicalMetallicRoughness(_) => todo!(),
            SceneMaterialType::Flat(_) => todo!(),
            SceneMaterialType::Foreign(_) => todo!(),
            _ => todo!(),
          }

          let primitive = gltf_json::mesh::Primitive {
            attributes: todo!(),
            indices: todo!(),
            material: todo!(),
            mode: gltf_json::validation::Checked::Valid(todo!()),
            targets: Default::default(),
            extensions: Default::default(),
            extras: Default::default(),
          };

          models.push(gltf_json::Mesh {
            extensions: Default::default(),
            extras: Default::default(),
            name: Default::default(),
            primitives: vec![primitive],
            weights: Default::default(),
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
    animations: Default::default(),
    asset: todo!(),
    buffers: todo!(),
    buffer_views: todo!(),
    scene: Default::default(),
    extensions: Default::default(),
    extras: Default::default(),
    extensions_used: Default::default(),
    extensions_required: Default::default(),
    cameras: todo!(),
    images: todo!(),
    materials,
    meshes: models,
    nodes,
    samplers: todo!(),
    scenes: vec![scene],
    skins: Default::default(),
    textures: todo!(),
  };

  let gltf_root_file_path = folder_path.join(file_name);
  let mut file = File::create(gltf_root_file_path).map_err(GltfExportErr::IO)?;

  json.to_writer(BufWriter::new(file));

  Ok(())
}
