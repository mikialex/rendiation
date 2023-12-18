#![feature(option_get_or_insert_default)]

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::BufWriter;
use std::ops::Deref;
use std::path::Path;

use fast_hash_collection::*;
use gltf_json::Root;
use rendiation_scene_core::*;
use rendiation_texture::TextureSampler;

mod convert_utils;
use convert_utils::*;

#[derive(Debug)]
pub enum GltfExportErr {
  IO(std::io::Error),
  Serialize(Box<dyn std::error::Error>),
}

pub fn build_scene_to_gltf(
  scene: &Scene,
  folder_path: &Path,
  file_name: &str,
) -> Result<(), GltfExportErr> {
  fs::create_dir_all(folder_path).map_err(GltfExportErr::IO)?;

  let ctx = Ctx::default();

  let mut scene_node_ids = Vec::default();
  let mut scene_index_map = FastHashMap::default();

  let scene_core = scene.get_scene_core();
  let scene = scene_core.read();
  let tree = scene.nodes.inner.inner.read();
  tree.expand_with_mapping(
    |node| (node.deref().clone(), node.guid()),
    |d| match d {
      tree::TreeExpandMutation::Create { data, node } => {
        let idx = ctx.build_node(&data.0, data.1);
        scene_node_ids.push(idx);
        scene_index_map.insert(node, idx);
      }
      tree::TreeExpandMutation::Attach {
        parent_target,
        node,
      } => {
        let node_idx = scene_index_map.get(&node).unwrap();
        let parent_idx = scene_index_map.get(&parent_target).unwrap();
        ctx.nodes.collected.borrow_mut()[parent_idx.value()]
          .children
          .get_or_insert_default()
          .push(*node_idx);
      }
    },
  );

  for (_, model) in &scene.models {
    let model = model.read();

    let mapping = ctx.nodes.mapping.borrow();
    let mut collected = ctx.nodes.collected.borrow_mut();
    let idx = mapping.get(&model.node.guid()).unwrap();
    let node = collected.get_mut(idx.value()).unwrap();
    node.mesh = ctx.build_model(&model.model)
  }

  // for (_, light) in &scene.lights {
  //   let light = light.read();
  //   match light.light {
  //     SceneLightKind::PointLight(_) => todo!(),
  //     SceneLightKind::SpotLight(_) => todo!(),
  //     SceneLightKind::DirectionalLight(_) => todo!(),
  //     _ => todo!(),
  //   }
  // }

  // for (_, camera) in &scene.cameras {
  //   let camera = camera.read();
  //   match camera.projection {
  //     CameraProjector::Perspective(_) => todo!(),
  //     CameraProjector::ViewOrthographic(_) => todo!(),
  //     CameraProjector::Orthographic(_) => todo!(),
  //     _ => todo!(),
  //   }
  // }

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
    min_version: String::from("2").into(), // is this mean the gltf version or exporter version??
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
  let binary_data = ctx.binary_data.borrow();
  let json_buf = json
    .to_vec()
    .map_err(|e| GltfExportErr::Serialize(Box::new(e)))?;

  // todo not optimal, improve upstream
  let glb = gltf::Glb {
    header: gltf::binary::Header {
      magic: *b"glTF",
      version: 2,
      length: 0, // this actually computed by writer when writing
    },
    json: Cow::Borrowed(&json_buf),
    bin: binary_data
      .as_ref()
      .map(|b| Cow::Borrowed(b.binary_data.as_slice())),
  };

  let file = File::create(gltf_root_file_path).map_err(GltfExportErr::IO)?;
  glb
    .to_writer(BufWriter::new(file))
    .map_err(|e| GltfExportErr::Serialize(Box::new(e)))
}

#[derive(Default)]
struct Ctx {
  nodes: Resource<u64, gltf_json::Node>,
  models: Resource<u64, gltf_json::Mesh>,
  cameras: Resource<u64, gltf_json::Camera>,

  materials: Resource<u64, gltf_json::Material>,
  images: Resource<u64, gltf_json::Image>,
  samplers: Resource<TextureSampler, gltf_json::texture::Sampler>,
  textures: Resource<(u64, TextureSampler), gltf_json::Texture>,

  binary_data: std::cell::RefCell<Option<InlineBinary>>,
  buffers: Resource<u64, gltf_json::Buffer>,
  buffer_views: Resource<ViewKey, gltf_json::buffer::View>,
  accessors: Resource<AttributeAccessorKey, gltf_json::Accessor>,
}

#[derive(PartialEq, Eq, Hash)]
struct ViewKey {
  pub buffer_id: u64,
  pub view_range: BufferViewRange,
}

#[derive(PartialEq, Eq, Hash)]
struct AttributeAccessorKey {
  pub view: usize,
  pub byte_offset: usize,
  pub count: usize,
  pub item_byte_size: usize,
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
  pub fn build_inline_accessor(
    &self,
    acc: &AttributeAccessor,
    c_ty: gltf_json::accessor::ComponentType,
    ty: gltf_json::accessor::Type,
    normalized: bool,
  ) -> Option<gltf_json::Index<gltf_json::Accessor>> {
    let view = self.buffer_views.get_or_insert_with(
      ViewKey {
        buffer_id: acc.view.buffer.guid(),
        view_range: acc.view.range,
      },
      || {
        let (buffer, byte_length, byte_offset) =
          self.collect_inline_buffer(&acc.view.buffer.read().buffer);
        gltf_json::buffer::View {
          buffer,
          byte_length,
          byte_offset: byte_offset.into(),
          byte_stride: Default::default(),
          name: Default::default(),
          target: Default::default(),
          extensions: Default::default(),
          extras: Default::default(),
        }
        .into()
      },
    )?;

    let key = AttributeAccessorKey {
      view: view.value(),
      byte_offset: acc.byte_offset,
      count: acc.count,
      item_byte_size: acc.item_byte_size,
    };

    self.accessors.get_or_insert_with(key, || {
      gltf_json::Accessor {
        buffer_view: view.into(),
        byte_offset: acc.byte_offset as u32,
        count: acc.count as u32,
        component_type: gltf_json::validation::Checked::Valid(
          gltf_json::accessor::GenericComponentType(c_ty),
        ),
        extensions: Default::default(),
        extras: Default::default(),
        type_: gltf_json::validation::Checked::Valid(ty),
        min: Default::default(),
        max: Default::default(),
        name: Default::default(),
        normalized,
        sparse: Default::default(),
      }
      .into()
    })
  }

  pub fn build_node(&self, node: &SceneNodeDataImpl, id: u64) -> gltf_json::Index<gltf_json::Node> {
    self
      .nodes
      .get_or_insert_with(id, || {
        gltf_json::Node {
          camera: Default::default(),
          children: Default::default(),
          extensions: Default::default(),
          extras: Default::default(),
          matrix: Some(node.local_matrix.into()),
          mesh: Default::default(),
          name: Default::default(),
          rotation: Default::default(),
          scale: Default::default(),
          translation: Default::default(),
          skin: Default::default(),
          weights: Default::default(),
        }
        .into()
      })
      .unwrap()
  }

  pub fn build_model(&self, model: &ModelEnum) -> Option<gltf_json::Index<gltf_json::Mesh>> {
    match model {
      ModelEnum::Standard(model) => {
        let model = model.read();
        match &model.mesh {
          MeshEnum::AttributesMesh(mesh) => {
            let mesh = mesh.read();
            self.models.get_or_insert_with(model.guid(), || {
              #[allow(clippy::disallowed_types)]
              let mut attributes = std::collections::BTreeMap::default();
              for (key, att) in &mesh.attributes {
                let (key, cty, ty) = map_semantic_att(key)?;
                let key = gltf_json::validation::Checked::Valid(key);
                attributes.insert(key, self.build_inline_accessor(att, cty, ty, false)?);
              }

              let primitive = gltf_json::mesh::Primitive {
                attributes,
                indices: match &mesh.indices {
                  Some((fmt, acc)) => match fmt {
                    AttributeIndexFormat::Uint16 => self.build_inline_accessor(
                      acc,
                      gltf_json::accessor::ComponentType::U16,
                      gltf_json::accessor::Type::Scalar,
                      false,
                    )?,
                    AttributeIndexFormat::Uint32 => self.build_inline_accessor(
                      acc,
                      gltf_json::accessor::ComponentType::U32,
                      gltf_json::accessor::Type::Scalar,
                      false,
                    )?,
                  }
                  .into(),
                  None => None,
                },
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
              .into()
            })
          }
          MeshEnum::TransformInstanced(_) => None,
          MeshEnum::Foreign(_) => None,
        }
      }
      _ => None,
    }
  }

  pub fn build_material(
    &self,
    material: &MaterialEnum,
  ) -> Option<gltf_json::Index<gltf_json::Material>> {
    match material {
      MaterialEnum::PhysicalSpecularGlossiness(_) => None,
      MaterialEnum::PhysicalMetallicRoughness(material) => {
        self.materials.get_or_insert_with(material.guid(), || {
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
                .and_then(|t| self.build_texture2d_info(t, 0)),
              metallic_factor: gltf_json::material::StrengthFactor(material.metallic),
              roughness_factor: gltf_json::material::StrengthFactor(material.roughness),
              metallic_roughness_texture: material
                .metallic_roughness_texture
                .as_ref()
                .and_then(|t| self.build_texture2d_info(t, 0)),
              ..Default::default()
            },
            normal_texture: material.normal_texture.as_ref().and_then(|t| {
              gltf_json::material::NormalTexture {
                index: self.build_texture2d(&t.content)?,
                scale: t.scale,
                tex_coord: 0,
                extensions: Default::default(),
                extras: Default::default(),
              }
              .into()
            }),
            occlusion_texture: None,
            emissive_texture: material
              .emissive_texture
              .as_ref()
              .and_then(|t| self.build_texture2d_info(t, 0)),
            emissive_factor: gltf_json::material::EmissiveFactor(material.emissive.into()),
            ..Default::default()
          }
          .into()
        })
      }
      MaterialEnum::Flat(_) => None,
      MaterialEnum::Foreign(_) => None,
    }
  }

  pub fn build_texture2d_info(
    &self,
    ts: &Texture2DWithSamplingData,
    tex_coord: usize,
  ) -> Option<gltf_json::texture::Info> {
    gltf_json::texture::Info {
      index: self.build_texture2d(ts)?,
      tex_coord: tex_coord as u32,
      extensions: Default::default(),
      extras: Default::default(),
    }
    .into()
  }
  pub fn build_texture2d(
    &self,
    ts: &Texture2DWithSamplingData,
  ) -> Option<gltf_json::Index<gltf_json::Texture>> {
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
        }.into(),
        SceneTexture2DType::Foreign(_) => None,
      }
    })?;

    let sampler_content = *ts.sampler.read();
    let sampler = self.samplers.get_or_insert_with(sampler_content, || {
      map_sampler(sampler_content, true).into()
    })?;

    self
      .textures
      .get_or_insert_with((ts.texture.guid(), sampler_content), || {
        gltf_json::Texture {
          name: Default::default(),
          sampler: Some(sampler),
          source,
          extensions: Default::default(),
          extras: Default::default(),
        }
        .into()
      })
  }
}

struct Resource<K, T> {
  collected: std::cell::RefCell<Vec<T>>,
  mapping: std::cell::RefCell<FastHashMap<K, gltf_json::Index<T>>>,
}

impl<K, T> Resource<K, T> {
  pub fn append_and_skip_mapping(&self, v: T) -> gltf_json::Index<T> {
    let idx = self.collected.borrow().len();
    self.collected.borrow_mut().push(v);
    gltf_json::Index::new(idx as u32)
  }

  pub fn get_or_insert_with(
    &self,
    key: K,
    create: impl FnOnce() -> Option<T>,
  ) -> Option<gltf_json::Index<T>>
  where
    K: std::hash::Hash + Eq,
  {
    let mut mapping = self.mapping.borrow_mut();
    if let Some(v) = mapping.get(&key) {
      return (*v).into();
    } else if let Some(v) = create() {
      let v = self.append_and_skip_mapping(v);
      mapping.insert(key, v);
      return v.into();
    }
    None
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
