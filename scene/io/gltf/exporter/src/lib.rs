#![feature(path_add_extension)]

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use database::*;
use fast_hash_collection::*;
use gltf_json::Root;
use rendiation_algebra::Vec3;
use rendiation_geometry::Box3;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_texture_core::TextureSampler;

mod convert_utils;
use convert_utils::*;

#[derive(Debug)]
pub enum GltfExportErr {
  IO(std::io::Error),
  Serialize(Box<dyn std::error::Error>),
}

pub fn build_scene_to_gltf(
  reader: SceneReader,
  folder_path: &Path,
  file_name: &str,
) -> Result<(), GltfExportErr> {
  fs::create_dir_all(folder_path).map_err(GltfExportErr::IO)?;

  let ctx = Ctx {
    nodes: Default::default(),
    models: Default::default(),
    cameras: Default::default(),
    materials: Default::default(),
    images: Default::default(),
    samplers: Default::default(),
    textures: Default::default(),
    binary_data: Default::default(),
    buffers: Default::default(),
    buffer_views: Default::default(),
    accessors: Default::default(),
    reader,
  };

  let mut scene_node_ids = Vec::default();
  let mut scene_index_map = FastHashMap::default();

  let mut all_nodes = FastHashSet::default();
  let mut batch_writes = Vec::default();
  for model in ctx.reader.models() {
    let model_info = ctx.reader.read_scene_model(model);
    all_nodes.insert(model_info.node);
  }
  let first_batch = all_nodes.iter().copied().collect::<Vec<_>>();
  batch_writes.push(first_batch);

  loop {
    let mut new_batch = Vec::default();
    for node in batch_writes.last().unwrap() {
      if let Some(parent) = ctx.reader.read_node_parent(*node) {
        if !all_nodes.contains(&parent) {
          all_nodes.insert(parent);
          new_batch.push(parent);
        }
      }
    }
    if new_batch.is_empty() {
      break;
    }
    batch_writes.push(new_batch);
  }

  for batch in batch_writes.iter().rev() {
    for node in batch {
      let idx = ctx.build_node(*node);
      scene_node_ids.push(idx);
      scene_index_map.insert(*node, idx);
      if let Some(parent) = ctx.reader.read_node_parent(*node) {
        let node_idx = scene_index_map.get(node).unwrap();
        let parent_idx = scene_index_map.get(&parent).unwrap();
        ctx.nodes.collected.borrow_mut()[parent_idx.value()]
          .children
          .get_or_insert_default()
          .push(*node_idx);
      }
    }
  }

  for model in ctx.reader.models() {
    let model_info = ctx.reader.read_scene_model(model);

    let mapping = ctx.nodes.mapping.borrow();
    let mut collected = ctx.nodes.collected.borrow_mut();
    let idx = mapping.get(&model_info.node).unwrap();
    let node = collected.get_mut(idx.value()).unwrap();
    node.mesh = ctx.build_model(model)
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

  let mut gltf_root_file_path = folder_path.join(file_name);
  gltf_root_file_path.add_extension("glb");
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

struct Ctx {
  nodes: Resource<EntityHandle<SceneNodeEntity>, gltf_json::Node>,
  models: Resource<EntityHandle<SceneModelEntity>, gltf_json::Mesh>,
  cameras: Resource<EntityHandle<SceneCameraEntity>, gltf_json::Camera>,

  materials: Resource<EntityHandle<PbrMRMaterialEntity>, gltf_json::Material>,
  images: Resource<EntityHandle<SceneTexture2dEntity>, gltf_json::Image>,
  samplers: Resource<TextureSampler, gltf_json::texture::Sampler>,
  textures: Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,

  binary_data: std::cell::RefCell<Option<InlineBinary>>,
  buffers: Resource<u64, gltf_json::Buffer>,
  buffer_views: Resource<ViewKey, gltf_json::buffer::View>,
  accessors: Resource<AttributeAccessorKey, gltf_json::Accessor>,

  reader: SceneReader,
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
        gltf_json::validation::USize64(binary_data.binary_data.len() as u64);
    }
  }
}

impl Ctx {
  // return (id, len, offset)
  pub fn collect_inline_buffer(
    &self,
    buffer: &[u8],
  ) -> (
    gltf_json::Index<gltf_json::Buffer>,
    gltf_json::validation::USize64,
    Option<gltf_json::validation::USize64>,
  ) {
    let mut binary = self.binary_data.borrow_mut();
    let binary = binary.get_or_insert_with(|| InlineBinary {
      binary_data: Default::default(),
      idx: self.buffers.append_and_skip_mapping(gltf_json::Buffer {
        byte_length: gltf_json::validation::USize64(0),
        name: Default::default(),
        uri: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
      }),
    });
    let byte_len = buffer.len();
    let byte_offset = binary.binary_data.len();
    binary.binary_data.extend_from_slice(buffer);

    (
      binary.idx,
      gltf_json::validation::USize64(byte_len as u64),
      gltf_json::validation::USize64(byte_offset as u64).into(),
    )
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
        byte_offset,
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
        buffer_id: acc.view.buffer.as_ptr() as u64,
        view_range: acc.view.range,
      },
      || {
        let (buffer, byte_length, byte_offset) = self.collect_inline_buffer(&acc.view.buffer);
        gltf_json::buffer::View {
          buffer,
          byte_length,
          byte_offset,
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
        byte_offset: gltf_json::validation::USize64(acc.byte_offset as u64).into(),
        count: gltf_json::validation::USize64(acc.count as u64),
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

  pub fn build_node(&self, id: EntityHandle<SceneNodeEntity>) -> gltf_json::Index<gltf_json::Node> {
    self
      .nodes
      .get_or_insert_with(id, || {
        let node = self.reader.read_node(id);
        gltf_json::Node {
          camera: Default::default(),
          children: Default::default(),
          extensions: Default::default(),
          extras: Default::default(),
          matrix: Some(node.local_matrix.into_f32().into()),
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

  pub fn build_model(
    &self,
    id: EntityHandle<SceneModelEntity>,
  ) -> Option<gltf_json::Index<gltf_json::Mesh>> {
    self.models.get_or_insert_with(id, || {
      let model = self.reader.read_scene_model(id);
      let std_model = self.reader.read_std_model(model.model);
      let mesh = self.reader.read_attribute_mesh(std_model.mesh);

      #[allow(clippy::disallowed_types)]
      let mut attributes = std::collections::BTreeMap::default();
      for (key, att) in &mesh.attributes {
        let (key_, cty, ty) = map_semantic_att(key)?;

        let key = gltf_json::validation::Checked::Valid(key_.clone());

        let acc = self.build_inline_accessor(att, cty, ty, false)?;

        // todo, consider using scene derive data result
        if key_ == gltf_json::mesh::Semantic::Positions {
          let att = att.read();
          let bbox: Box3 = att.visit_slice::<Vec3<f32>>().unwrap().iter().collect();

          fn vec3_to_gltf(v: Vec3<f32>) -> gltf_json::Value {
            gltf_json::Value::Array(vec![
              serde_json::json!(v.x),
              serde_json::json!(v.y),
              serde_json::json!(v.z),
            ])
          }

          self.accessors.mutate(acc, |att| {
            att.max = Some(vec3_to_gltf(bbox.max));
            att.min = Some(vec3_to_gltf(bbox.min));
          })
        }

        attributes.insert(key, acc);
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
        material: self.build_material(&std_model.material),
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

  pub fn build_material(
    &self,
    material: &SceneMaterialDataView,
  ) -> Option<gltf_json::Index<gltf_json::Material>> {
    match material {
      SceneMaterialDataView::PbrMRMaterial(material) => {
        self.materials.get_or_insert_with(*material, || {
          let material = self.reader.read_pbr_mr_material(*material);
          gltf_json::Material {
            alpha_cutoff: gltf_json::material::AlphaCutoff(material.alpha.alpha_cutoff).into(),
            alpha_mode: gltf_json::validation::Checked::Valid(map_alpha_mode(
              material.alpha.alpha_mode,
            )),
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
      _ => None,
    }
  }

  pub fn build_texture2d_info(
    &self,
    ts: &Texture2DWithSamplingDataView,
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
    ts: &Texture2DWithSamplingDataView,
  ) -> Option<gltf_json::Index<gltf_json::Texture>> {
    let source = self.images.get_or_insert_with(ts.texture, || {
      let texture = self.reader.read_texture(ts.texture);
      // todo set mime type and encode png
      gltf_json::Image {
        buffer_view: self.collect_inline_packed_view_buffer(&texture.data).into(),
        mime_type: Default::default(),
        name: Default::default(),
        uri: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
      }
      .into()
    })?;

    let sampler_content = self.reader.read_sampler(ts.sampler);
    let sampler = self.samplers.get_or_insert_with(sampler_content, || {
      map_sampler(sampler_content, true).into()
    })?;

    self
      .textures
      .get_or_insert_with((ts.texture, sampler_content), || {
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

  pub fn mutate(&self, idx: gltf_json::Index<T>, f: impl FnOnce(&mut T)) {
    let mut collected = self.collected.borrow_mut();
    let v = &mut collected[idx.value()];
    f(v);
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
