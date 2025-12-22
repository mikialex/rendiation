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

mod material;
use material::*;

mod buffer_inliner;
use buffer_inliner::*;

mod resource_collector;
use resource_collector::*;

#[derive(Debug)]
pub enum GltfExportErr {
  IO(std::io::Error),
  Serialize(Box<dyn std::error::Error>),
}

/// Implementation Note
///
/// The gltf is a node forest, each node can only attach one model(gltf mesh), but in our scene structure
/// a node can have multiple models, so we create a new node for node that has multiple model when export.
pub fn build_scene_to_gltf(
  reader: &SceneReader,
  folder_path: &Path,
  file_name: &str,
) -> Result<(), GltfExportErr> {
  fs::create_dir_all(folder_path).map_err(GltfExportErr::IO)?;

  // write nodes
  let mut target_nodes = Resource::default();
  let mut root_nodes = Vec::default();
  {
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
        scene_index_map.insert(*node, idx);
        if let Some(parent) = reader.read_node_parent(*node) {
          let node_idx = scene_index_map.get(node).unwrap();
          let parent_idx = scene_index_map.get(&parent).unwrap();
          target_nodes.collected[parent_idx.value()]
            .children
            .get_or_insert_default()
            .push(*node_idx);
        } else {
          root_nodes.push(idx);
        }
      }
    }
  };

  let mut all_texture_to_write = FastHashSet::default();

  let mut all_mesh_to_write = FastHashSet::default();

  let mut all_material_to_write = FastHashSet::default();

  let mut has_sg_material = false;

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
        has_sg_material = true;
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
      _ => {}
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
    let idx = *target_nodes.mapping.get(&model_info.node).unwrap();
    let node = target_nodes.collected.get_mut(idx.value()).unwrap();
    let targe_node_idx = if node.mesh.is_some() {
      let extra_node = build_node(&mut target_nodes, reader, model_info.node);
      if let Some(parent) = reader.read_node_parent(model_info.node) {
        let parent_idx = target_nodes.get(&parent);
        target_nodes.collected[parent_idx.value()]
          .children
          .get_or_insert_default()
          .push(extra_node);
      } else {
        root_nodes.push(extra_node);
      }
      extra_node
    } else {
      idx
    };

    let node = target_nodes
      .collected
      .get_mut(targe_node_idx.value())
      .unwrap();

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
    nodes: root_nodes,
    extensions: Default::default(),
    extras: Default::default(),
    name: Default::default(),
  };

  let asset = gltf_json::Asset {
    copyright: None,
    extensions: Default::default(),
    extras: Default::default(),
    generator: String::from("rendiation_scene_gltf_exporter").into(),
    min_version: String::from("2.0").into(),
    version: String::from("2.0"),
  };

  let mut extensions_used = Vec::new();
  if has_sg_material {
    extensions_used.push(String::from("KHR_materials_pbrSpecularGlossiness"));
  }

  let json = gltf_json::Root {
    accessors: accessors.collected,
    animations: Default::default(),
    asset,
    buffers: buffers.collected,
    buffer_views: buffer_views.collected,
    scene: Default::default(),
    extensions: Default::default(),
    extras: Default::default(),
    extensions_used,
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
  buffer_view_map: &mut FastHashMap<ViewKey, gltf_json::Index<gltf_json::buffer::View>>,
  accessors: &mut Resource<AttributeAccessorKey, gltf_json::Accessor>,
  acc: &AttributeAccessor,
  c_ty: gltf_json::accessor::ComponentType,
  ty: gltf_json::accessor::Type,
  normalized: bool,
) -> gltf_json::Index<gltf_json::Accessor> {
  let view = buffer_view_map
    .entry(ViewKey {
      buffer_id: acc.view.buffer.as_ptr() as u64,
      view_range: acc.view.range,
    })
    .or_insert_with(|| buffer_inliner.collect_inline_packed_view_buffer(&acc.view.buffer));

  let key = AttributeAccessorKey {
    view: view.value(),
    byte_offset: acc.byte_offset,
    count: acc.count,
    item_byte_size: acc.item_byte_size,
  };

  accessors.get_or_insert_with(key, || gltf_json::Accessor {
    buffer_view: (*view).into(),
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
  buffer_view_map: &mut FastHashMap<ViewKey, gltf_json::Index<gltf_json::buffer::View>>,
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

        let acc = build_inline_accessor(
          buffer_inliner,
          buffer_view_map,
          accessors,
          att,
          cty,
          ty,
          false,
        );

        // todo, consider using scene derive data result
        if key_ == gltf_json::mesh::Semantic::Positions {
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
              buffer_view_map,
              accessors,
              acc,
              gltf_json::accessor::ComponentType::U16,
              gltf_json::accessor::Type::Scalar,
              false,
            ),
            AttributeIndexFormat::Uint32 => build_inline_accessor(
              buffer_inliner,
              buffer_view_map,
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
