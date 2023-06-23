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

pub fn build_scene_to_gltf(
  scene: &Scene,
  folder_path: &Path,
  file_name: &str,
) -> Result<(), GltfExportErr> {
  fs::create_dir_all(folder_path).map_err(GltfExportErr::IO)?;

  let scene = scene.read();

  let mut scene_node_ids = Vec::default();

  // todo load scene.nodes.

  let ctx = Ctx::default();

  for (_, model) in &scene.models {
    let model = model.read();
    // let node_idx = *node_mapping.get(&model.node.guid()).unwrap();
    // let node = nodes.get_mut(node_idx).unwrap();
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

  let asset = gltf_json::Asset {
    copyright: None,
    extensions: Default::default(),
    extras: Default::default(),
    generator: String::from("rendiation_scene_gltf_exporter").into(),
    min_version: String::from("2").into(),
    version: String::from("2"),
  };

  ctx.finalize();

  let json = Root {
    accessors: ctx.accessors.collected.take(),
    animations: Default::default(),
    asset,
    buffers: ctx.buffers.collected.take(),
    buffer_views: ctx.buffer_views.collected.take(),
    scene: Default::default(),
    extensions: Default::default(),
    extras: Default::default(),
    extensions_used: Default::default(),
    extensions_required: Default::default(),
    cameras: ctx.cameras.collected.take(),
    images: ctx.images.collected.take(),
    materials: ctx.materials.collected.take(),
    meshes: ctx.models.collected.take(),
    nodes: ctx.nodes.collected.take(),
    samplers: ctx.samplers.collected.take(),
    scenes: vec![scene],
    skins: Default::default(),
    textures: ctx.textures.collected.take(),
  };

  let gltf_root_file_path = folder_path.join(file_name);
  let file = File::create(gltf_root_file_path).map_err(GltfExportErr::IO)?;

  json.to_writer(BufWriter::new(file));

  Ok(())
}

#[derive(Default)]
struct Ctx {
  nodes: Resource<usize, gltf_json::Node>,
  models: Resource<usize, gltf_json::Mesh>,
  cameras: Resource<usize, gltf_json::Camera>,

  materials: Resource<usize, gltf_json::Material>,
  images: Resource<usize, gltf_json::Image>,
  samplers: Resource<TextureSampler, gltf_json::texture::Sampler>,
  textures: Resource<(usize, TextureSampler), gltf_json::Texture>,

  binary_data: std::cell::RefCell<Option<InlineBinary>>,
  buffers: Resource<usize, gltf_json::Buffer>,
  buffer_views: Resource<usize, gltf_json::buffer::View>,
  accessors: Resource<usize, gltf_json::Accessor>,
}

struct InlineBinary {
  binary_data: Vec<u8>,
  idx: gltf_json::Index<gltf_json::Buffer>,
}

impl Ctx {
  fn finalize(&self) {
    let b = self.binary_data.borrow();
    let b: &Option<InlineBinary> = &b;
    if let Some(binary_data) = b {
      self.buffers.collected.borrow_mut()[binary_data.idx.value()].byte_length =
        binary_data.binary_data.len() as u32;
    }
  }
}

impl Ctx {
  // return (id, len, offset)
  pub fn collect_inline_buffer(
    &self,
    buffer: &[u8],
  ) -> (gltf_json::Index<gltf_json::Buffer>, u32, u32) {
    let mut binary = self.binary_data.borrow_mut();
    let binary = binary.get_or_insert_with(|| InlineBinary {
      binary_data: Default::default(),
      idx: self.buffers.append_and_skip_mapping(gltf_json::Buffer {
        byte_length: 0,
        name: Default::default(),
        uri: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
      }),
    });
    let byte_len = buffer.len();
    let byte_offset = binary.binary_data.len();
    binary.binary_data.extend_from_slice(buffer);

    (binary.idx, byte_len as u32, byte_offset as u32)
  }

  pub fn collect_inline_packed_view_buffer(
    &self,
    buffer: &[u8],
  ) -> gltf_json::Index<gltf_json::buffer::View> {
    let (buffer, byte_length, byte_offset) = self.collect_inline_buffer(buffer);
    self
      .buffer_views
      .append_and_skip_mapping(gltf_json::buffer::View {
        buffer,
        byte_length,
        byte_offset: byte_offset.into(),
        byte_stride: Default::default(),
        name: Default::default(),
        target: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
      })
  }

  pub fn build_node(&self, node: &SceneNode) -> gltf_json::Index<gltf_json::Node> {
    node.visit(|node| {
      self.nodes.get_or_insert_with(node.guid(), || {
        //
        gltf_json::Node {
          camera: Default::default(),
          children: todo!(),
          extensions: Default::default(),
          extras: Default::default(),
          matrix: todo!(),
          mesh: Default::default(),
          name: Default::default(),
          rotation: todo!(),
          scale: todo!(),
          translation: todo!(),
          skin: Default::default(),
          weights: Default::default(),
        }
      })
    })
  }

  pub fn build_model(&self, model: &ModelType) -> Option<gltf_json::Index<gltf_json::Mesh>> {
    match model {
      ModelType::Standard(model) => {
        let model = model.read();
        match &model.mesh {
          SceneMeshType::AttributesMesh(mesh) => {
            let mesh = mesh.read();
            self
              .models
              .get_or_insert_with(model.guid(), || {
                let primitive = gltf_json::mesh::Primitive {
                  attributes: todo!(),
                  indices: todo!(),
                  material: self.build_material(&model.material),
                  mode: gltf_json::validation::Checked::Valid(map_draw_mode(mesh.mode)),
                  targets: Default::default(),
                  extensions: Default::default(),
                  extras: Default::default(),
                };

                gltf_json::Mesh {
                  extensions: Default::default(),
                  extras: Default::default(),
                  name: Default::default(),
                  primitives: vec![primitive],
                  weights: Default::default(),
                }
              })
              .into()
          }
          SceneMeshType::TransformInstanced(_) => None,
          SceneMeshType::Foreign(_) => None,
          _ => None,
        }
      }
      _ => None,
    }
  }

  pub fn build_material(
    &self,
    material: &SceneMaterialType,
  ) -> Option<gltf_json::Index<gltf_json::Material>> {
    match material {
      SceneMaterialType::PhysicalSpecularGlossiness(material) => None,
      SceneMaterialType::PhysicalMetallicRoughness(material) => self
        .materials
        .get_or_insert_with(material.guid(), || {
          let material = material.read();
          gltf_json::Material {
            alpha_cutoff: gltf_json::material::AlphaCutoff(material.alpha_cutoff).into(),
            alpha_mode: gltf_json::validation::Checked::Valid(map_alpha_mode(material.alpha_mode)),
            double_sided: false,
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
                .map(|t| self.build_texture2d_info(t, 0)),
              metallic_factor: gltf_json::material::StrengthFactor(material.metallic),
              roughness_factor: gltf_json::material::StrengthFactor(material.roughness),
              metallic_roughness_texture: material
                .metallic_roughness_texture
                .as_ref()
                .map(|t| self.build_texture2d_info(t, 0)),
              ..Default::default()
            },
            normal_texture: material.normal_texture.as_ref().map(|t| {
              gltf_json::material::NormalTexture {
                index: self.build_texture2d(&t.content),
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
              .map(|t| self.build_texture2d_info(t, 0)),
            emissive_factor: gltf_json::material::EmissiveFactor(material.emissive.into()),
            ..Default::default()
          }
        })
        .into(),
      SceneMaterialType::Flat(_) => None,
      SceneMaterialType::Foreign(_) => None,
      _ => None,
    }
  }

  pub fn build_texture2d_info(
    &self,
    ts: &Texture2DWithSamplingData,
    tex_coord: usize,
  ) -> gltf_json::texture::Info {
    gltf_json::texture::Info {
      index: self.build_texture2d(ts),
      tex_coord: tex_coord as u32,
      extensions: Default::default(),
      extras: Default::default(),
    }
  }
  pub fn build_texture2d(
    &self,
    ts: &Texture2DWithSamplingData,
  ) -> gltf_json::Index<gltf_json::Texture> {
    let source = self.images.get_or_insert_with(ts.texture.guid(), || {
      let texture = ts.texture.read();
      let texture: &SceneTexture2DType = &texture;
      match texture {
        SceneTexture2DType::GPUBufferImage(image) => gltf_json::Image {
          buffer_view: self.collect_inline_packed_view_buffer(&image.data).into(),// todo set mime type and encode png
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
        source,
        extensions: Default::default(),
        extras: Default::default(),
      })
  }
}

struct Resource<K, T> {
  collected: std::cell::RefCell<Vec<T>>,
  mapping: std::cell::RefCell<HashMap<K, gltf_json::Index<T>>>,
}

impl<K, T> Resource<K, T> {
  pub fn append_and_skip_mapping(&self, v: T) -> gltf_json::Index<T> {
    let idx = self.collected.borrow().len();
    self.collected.borrow_mut().push(v);
    gltf_json::Index::new(idx as u32)
  }

  pub fn get_or_insert_with(&self, key: K, create: impl FnOnce() -> T) -> gltf_json::Index<T>
  where
    K: std::hash::Hash + Eq,
  {
    *self
      .mapping
      .borrow_mut()
      .entry(key)
      .or_insert_with(|| self.append_and_skip_mapping(create()))
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
