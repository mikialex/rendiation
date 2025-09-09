#![feature(path_add_extension)]

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use database::*;
use fast_hash_collection::*;
use rendiation_algebra::Vec3;
use rendiation_geometry::Box3;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_texture_core::TextureSampler;

mod convert_utils;
use convert_utils::*;

mod buffer_inliner;
use buffer_inliner::*;

mod resource_collector;
use resource_collector::*;

#[derive(Debug)]
pub enum GltfExportErr {
  IO(std::io::Error),
  Serialize(Box<dyn std::error::Error>),
}

pub fn build_scene_to_gltf(
  reader: &SceneReader,
  folder_path: &Path,
  file_name: &str,
) -> Result<(), GltfExportErr> {
  fs::create_dir_all(folder_path).map_err(GltfExportErr::IO)?;

  // write nodes
  let (scene_node_ids, mut target_nodes) = {
    let mut target_nodes = Resource::default();

    let mut scene_node_ids = Vec::default();
    let mut scene_index_map = FastHashMap::default();

    let mut all_nodes = FastHashSet::default();
    let mut batch_writes = Vec::default();
    for model in reader.models() {
      let model_info = reader.read_scene_model(model);
      all_nodes.insert(model_info.node);
    }
    let first_batch = all_nodes.iter().copied().collect::<Vec<_>>();
    batch_writes.push(first_batch);

    loop {
      let mut new_batch = Vec::default();
      for node in batch_writes.last().unwrap() {
        if let Some(parent) = reader.read_node_parent(*node) {
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
        let idx = build_node(&mut target_nodes, reader, *node);
        scene_node_ids.push(idx);
        scene_index_map.insert(*node, idx);
        if let Some(parent) = reader.read_node_parent(*node) {
          let node_idx = scene_index_map.get(node).unwrap();
          let parent_idx = scene_index_map.get(&parent).unwrap();
          target_nodes.collected[parent_idx.value()]
            .children
            .get_or_insert_default()
            .push(*node_idx);
        }
      }
    }
    (scene_node_ids, target_nodes)
  };

  let mut all_texture_to_write = FastHashSet::default();

  let mut all_mesh_to_write = FastHashSet::default();

  let mut all_material_to_write = FastHashSet::default();

  for model in reader.models() {
    let model_info = reader.read_scene_model(model);
    let std_model = reader.read_std_model(model_info.model);
    all_mesh_to_write.insert(std_model.mesh);

    match std_model.material {
      SceneMaterialDataView::UnlitMaterial(m) => {
        all_material_to_write.insert(std_model.material);
        let m = reader.read_unlit_material(m);
        if let Some(t) = m.color_alpha_tex {
          all_texture_to_write.insert(t);
        }
      }
      SceneMaterialDataView::PbrSGMaterial(m) => {
        all_material_to_write.insert(std_model.material);
        let m = reader.read_pbr_sg_material(m);
        if let Some(t) = m.albedo_texture {
          all_texture_to_write.insert(t);
        }
        if let Some(t) = m.normal_texture {
          all_texture_to_write.insert(t.content);
        }
        if let Some(t) = m.specular_glossiness_texture {
          all_texture_to_write.insert(t);
        }
        if let Some(t) = m.emissive_texture {
          all_texture_to_write.insert(t);
        }
      }
      SceneMaterialDataView::PbrMRMaterial(m) => {
        all_material_to_write.insert(std_model.material);
        let m = reader.read_pbr_mr_material(m);
        if let Some(t) = m.base_color_texture {
          all_texture_to_write.insert(t);
        }
        if let Some(t) = m.normal_texture {
          all_texture_to_write.insert(t.content);
        }
        if let Some(t) = m.metallic_roughness_texture {
          all_texture_to_write.insert(t);
        }
        if let Some(t) = m.emissive_texture {
          all_texture_to_write.insert(t);
        }
      }
    }
  }

  let mut keyed_buffer_views = Default::default();
  let mut accessors = Default::default();
  let mut models = Default::default();

  let mut binary_data = None;
  let mut buffers = Default::default();
  let mut buffer_views = Default::default();
  let mut buffer_builder = BufferResourceInliner {
    binary_data: &mut binary_data,
    buffers: &mut buffers,
    buffer_views: &mut buffer_views,
  };

  let mut images = Default::default();
  let mut samplers = Default::default();
  let mut textures = Default::default();

  for tex in all_texture_to_write {
    build_texture2d(
      &mut images,
      &mut samplers,
      &mut textures,
      Some(&mut buffer_builder),
      reader,
      &tex,
    );
  }

  let mut materials = Default::default();
  for m in all_material_to_write {
    build_material(&mut materials, reader, &m, &textures);
  }

  for model in reader.models() {
    let model_info = reader.read_scene_model(model);
    let idx = target_nodes.mapping.get(&model_info.node).unwrap();
    let node = target_nodes.collected.get_mut(idx.value()).unwrap();
    node.mesh = build_model(
      &mut buffer_builder,
      &mut keyed_buffer_views,
      &mut accessors,
      &mut models,
      &mut materials,
      reader,
      model,
    );
  }

  buffer_builder.finalize();

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

  let json = gltf_json::Root {
    accessors: accessors.collected,
    animations: Default::default(),
    asset,
    buffers: buffers.collected,
    buffer_views: buffer_views.collected,
    scene: Default::default(),
    extensions: Default::default(),
    extras: Default::default(),
    extensions_used: Default::default(),
    extensions_required: Default::default(),
    cameras: Default::default(),
    materials: materials.collected,
    meshes: models.collected,
    nodes: target_nodes.collected,
    images: images.collected,
    samplers: samplers.collected,
    textures: textures.collected,
    scenes: vec![scene],
    skins: Default::default(),
  };

  let mut gltf_root_file_path = folder_path.join(file_name);
  gltf_root_file_path.add_extension("glb");
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

#[derive(PartialEq, Eq, Hash)]
struct ViewKey {
  buffer_id: u64,
  view_range: BufferViewRange,
}

#[derive(PartialEq, Eq, Hash)]
struct AttributeAccessorKey {
  view: usize,
  byte_offset: usize,
  count: usize,
  item_byte_size: usize,
}

fn build_inline_accessor(
  buffer_inliner: &mut BufferResourceInliner,
  buffer_views: &mut Resource<ViewKey, gltf_json::buffer::View>,
  accessors: &mut Resource<AttributeAccessorKey, gltf_json::Accessor>,
  acc: &AttributeAccessor,
  c_ty: gltf_json::accessor::ComponentType,
  ty: gltf_json::accessor::Type,
  normalized: bool,
) -> gltf_json::Index<gltf_json::Accessor> {
  let view = buffer_views.get_or_insert_with(
    ViewKey {
      buffer_id: acc.view.buffer.as_ptr() as u64,
      view_range: acc.view.range,
    },
    || {
      let (buffer, byte_length, byte_offset) =
        buffer_inliner.collect_inline_buffer(&acc.view.buffer);
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
    },
  );

  let key = AttributeAccessorKey {
    view: view.value(),
    byte_offset: acc.byte_offset,
    count: acc.count,
    item_byte_size: acc.item_byte_size,
  };

  accessors.get_or_insert_with(key, || gltf_json::Accessor {
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
  })
}

fn build_node(
  nodes: &mut Resource<EntityHandle<SceneNodeEntity>, gltf_json::Node>,
  reader: &SceneReader,
  id: EntityHandle<SceneNodeEntity>,
) -> gltf_json::Index<gltf_json::Node> {
  let node = reader.read_node(id);
  let node = gltf_json::Node {
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
  };

  nodes.append(id, node)
}

fn build_model(
  buffer_inliner: &mut BufferResourceInliner,
  buffer_views: &mut Resource<ViewKey, gltf_json::buffer::View>,
  accessors: &mut Resource<AttributeAccessorKey, gltf_json::Accessor>,
  models: &mut Resource<EntityHandle<SceneModelEntity>, gltf_json::Mesh>,
  materials: &mut Resource<SceneMaterialDataView, gltf_json::Material>,
  reader: &SceneReader,
  id: EntityHandle<SceneModelEntity>,
) -> Option<gltf_json::Index<gltf_json::Mesh>> {
  models
    .append(id, {
      let model = reader.read_scene_model(id);
      let std_model = reader.read_std_model(model.model);
      let mesh = reader.read_attribute_mesh(std_model.mesh);

      #[allow(clippy::disallowed_types)]
      let mut attributes = std::collections::BTreeMap::default();
      for (key, att) in &mesh.attributes {
        let (key_, cty, ty) = map_semantic_att(key).unwrap();

        let key = gltf_json::validation::Checked::Valid(key_.clone());

        let acc =
          build_inline_accessor(buffer_inliner, buffer_views, accessors, att, cty, ty, false);

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

          accessors.mutate(acc, |att| {
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
            AttributeIndexFormat::Uint16 => build_inline_accessor(
              buffer_inliner,
              buffer_views,
              accessors,
              acc,
              gltf_json::accessor::ComponentType::U16,
              gltf_json::accessor::Type::Scalar,
              false,
            ),
            AttributeIndexFormat::Uint32 => build_inline_accessor(
              buffer_inliner,
              buffer_views,
              accessors,
              acc,
              gltf_json::accessor::ComponentType::U32,
              gltf_json::accessor::Type::Scalar,
              false,
            ),
          }
          .into(),
          None => None,
        },
        material: materials.try_get(&std_model.material),
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

fn build_material(
  materials: &mut Resource<SceneMaterialDataView, gltf_json::Material>,
  reader: &SceneReader,
  m: &SceneMaterialDataView,
  textures: &Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,
) -> Option<gltf_json::Index<gltf_json::Material>> {
  match m {
    SceneMaterialDataView::PbrMRMaterial(material) => materials
      .append(*m, {
        let material = reader.read_pbr_mr_material(*material);
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
              .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
            metallic_factor: gltf_json::material::StrengthFactor(material.metallic),
            roughness_factor: gltf_json::material::StrengthFactor(material.roughness),
            metallic_roughness_texture: material
              .metallic_roughness_texture
              .as_ref()
              .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
            ..Default::default()
          },
          normal_texture: material.normal_texture.as_ref().and_then(|t| {
            gltf_json::material::NormalTexture {
              index: {
                let sampler_content = reader.read_sampler(t.content.sampler);
                textures.get(&(t.content.texture, sampler_content))
              },
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
            .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
          emissive_factor: gltf_json::material::EmissiveFactor(material.emissive.into()),
          ..Default::default()
        }
      })
      .into(),
    _ => None,
  }
}

fn get_texture2d_info(
  ts: &Texture2DWithSamplingDataView,
  tex_coord: usize,
  reader: &SceneReader,
  textures: &Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,
) -> Option<gltf_json::texture::Info> {
  let sampler_content = reader.read_sampler(ts.sampler);
  gltf_json::texture::Info {
    index: textures.get(&(ts.texture, sampler_content)),
    tex_coord: tex_coord as u32,
    extensions: Default::default(),
    extras: Default::default(),
  }
  .into()
}

fn build_texture2d(
  images: &mut Resource<EntityHandle<SceneTexture2dEntity>, gltf_json::Image>,
  samplers: &mut Resource<TextureSampler, gltf_json::texture::Sampler>,
  textures: &mut Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,
  data_writer: Option<&mut BufferResourceInliner>,
  reader: &SceneReader,
  ts: &Texture2DWithSamplingDataView,
) -> gltf_json::Index<gltf_json::Texture> {
  let source = images.append(ts.texture, {
    let texture = reader.read_texture(ts.texture);

    let mut png_buffer = Vec::new(); // todo avoid extra copy
    rendiation_texture_exporter::write_gpu_buffer_image_as_png(&mut png_buffer, &texture);

    // todo set mime type and encode png
    let mut image = gltf_json::Image {
      buffer_view: Default::default(),
      mime_type: Some(gltf_json::image::MimeType("image/png".to_string())),
      name: Default::default(),
      uri: Default::default(),
      extensions: Default::default(),
      extras: Default::default(),
    };

    if let Some(data_writer) = data_writer {
      image.buffer_view = data_writer
        .collect_inline_packed_view_buffer(&texture.data)
        .into();
    }

    image
  });

  let sampler_content = reader.read_sampler(ts.sampler);
  let sampler = samplers.get_or_insert_with(sampler_content, || map_sampler(sampler_content, true));

  textures.get_or_insert_with((ts.texture, sampler_content), || gltf_json::Texture {
    name: Default::default(),
    sampler: sampler.into(),
    source,
    extensions: Default::default(),
    extras: Default::default(),
  })
}
