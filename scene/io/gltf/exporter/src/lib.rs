use std::fs::{self, File};
use std::io::BufWriter;
use std::{collections::HashMap, path::Path};

use gltf_json::Root;
use rendiation_scene_core::*;
use rendiation_texture::TextureSampler;

mod convert_utils;
use convert_utils::*;

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

  let mut ctx = Ctx::default();

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
            SceneMaterialType::PhysicalSpecularGlossiness(material) => {}
            SceneMaterialType::PhysicalMetallicRoughness(material) => {
              material_mapping.entry(material.guid()).or_insert_with(|| {
                let idx = materials.len();

                let material = material.read();
                materials.push(gltf_json::Material {
                  alpha_cutoff: gltf_json::material::AlphaCutoff(material.alpha_cutoff).into(),
                  alpha_mode: gltf_json::validation::Checked::Valid(map_alpha_mode(
                    material.alpha_mode,
                  )),
                  double_sided: todo!(),
                  pbr_metallic_roughness: gltf_json::material::PbrMetallicRoughness {
                    base_color_factor: gltf_json::material::PbrBaseColorFactor([
                      material.base_color.x,
                      material.base_color.y,
                      material.base_color.z,
                      1.,
                    ]),
                    base_color_texture: material
                      .base_color_texture
                      .as_ref()
                      .map(|t| ctx.export_texture2d_info(t, 0)),
                    metallic_factor: gltf_json::material::StrengthFactor(material.metallic),
                    roughness_factor: gltf_json::material::StrengthFactor(material.roughness),
                    metallic_roughness_texture: material
                      .metallic_roughness_texture
                      .as_ref()
                      .map(|t| ctx.export_texture2d_info(t, 0)),
                    ..Default::default()
                  },
                  normal_texture: material.normal_texture.as_ref().map(|t| {
                    gltf_json::material::NormalTexture {
                      index: ctx.export_texture2d(&t.content),
                      scale: t.scale,
                      tex_coord: 0,
                      extensions: Default::default(),
                      extras: Default::default(),
                    }
                  }),
                  occlusion_texture: None,
                  emissive_texture: material
                    .emissive_texture
                    .as_ref()
                    .map(|t| ctx.export_texture2d_info(t, 0)),
                  emissive_factor: gltf_json::material::EmissiveFactor(material.emissive.into()),
                  ..Default::default()
                });
                gltf_json::Index::new(idx as u32)
              });
            }
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
    images: ctx.images.collected,
    materials,
    meshes: models,
    nodes,
    samplers: ctx.samplers.collected,
    scenes: vec![scene],
    skins: Default::default(),
    textures: ctx.textures.collected,
  };

  let gltf_root_file_path = folder_path.join(file_name);
  let mut file = File::create(gltf_root_file_path).map_err(GltfExportErr::IO)?;

  json.to_writer(BufWriter::new(file));

  Ok(())
}

#[derive(Default)]
struct Ctx {
  images: Resource<usize, gltf_json::Image>,
  samplers: Resource<TextureSampler, gltf_json::texture::Sampler>,
  textures: Resource<(usize, TextureSampler), gltf_json::Texture>,
  buffers: Resource<usize, gltf_json::Buffer>,
}

impl Ctx {
  pub fn export_texture2d_info(
    &mut self,
    ts: &Texture2DWithSamplingData,
    tex_coord: usize,
  ) -> gltf_json::texture::Info {
    gltf_json::texture::Info {
      index: self.export_texture2d(ts),
      tex_coord: tex_coord as u32,
      extensions: Default::default(),
      extras: Default::default(),
    }
  }
  pub fn export_texture2d(
    &mut self,
    ts: &Texture2DWithSamplingData,
  ) -> gltf_json::Index<gltf_json::Texture> {
    let image = self.images.get_or_insert_with(ts.texture.guid(), || {
      let texture = ts.texture.read();
      let texture: &SceneTexture2DType = &texture;
      match texture {
        SceneTexture2DType::GPUBufferImage(image) => gltf_json::Image {
          buffer_view: todo!(),
          mime_type: Default::default(),
          name: Default::default(),
          uri: Default::default(),
          extensions: Default::default(),
          extras: Default::default(),
        },
        SceneTexture2DType::Foreign(_) => todo!(),
        _ => todo!(),
      }
    });

    let sampler = self.samplers.get_or_insert_with(ts.sampler, || {
      gltf_json::texture::Sampler {
        //  mag_filter: Option<Checked<MagFilter>>,
        //  min_filter: Option<Checked<MinFilter>>,
        wrap_s: gltf_json::validation::Checked::Valid(map_wrapping(ts.sampler.address_mode_u)),
        wrap_t: gltf_json::validation::Checked::Valid(map_wrapping(ts.sampler.address_mode_v)),
        ..Default::default()
      }
    });

    self
      .textures
      .get_or_insert_with((ts.texture.guid(), ts.sampler), || gltf_json::Texture {
        name: Default::default(),
        sampler: Some(sampler),
        source: todo!(),
        extensions: Default::default(),
        extras: Default::default(),
      })
  }
}

struct Resource<K, T> {
  collected: Vec<T>,
  mapping: HashMap<K, gltf_json::Index<T>>,
}

impl<K, T> Resource<K, T> {
  pub fn get_or_insert_with(&mut self, key: K, create: impl FnOnce() -> T) -> gltf_json::Index<T>
  where
    K: std::hash::Hash + Eq,
  {
    *self.mapping.entry(key).or_insert_with(|| {
      let idx = self.collected.len();
      self.collected.push(create());
      gltf_json::Index::new(idx as u32)
    })
  }
}

impl<K, T> Default for Resource<K, T> {
  fn default() -> Self {
    Self {
      collected: Default::default(),
      mapping: Default::default(),
    }
  }
}
