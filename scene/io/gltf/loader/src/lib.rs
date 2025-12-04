use core::num::NonZeroU64;
use std::path::{Path, PathBuf};

use database::*;
use fast_hash_collection::*;
use gltf::Node;
use rendiation_algebra::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
mod convert_utils;
use convert_utils::*;
use rendiation_texture_core::*;
use storage::IndexKeptVec;

const SUPPORTED_GLTF_EXTENSIONS: [&str; 3] = [
  "KHR_materials_pbrSpecularGlossiness",
  "KHR_lights_punctual",
  "KHR_materials_unlit",
];

#[derive(Debug)]
pub enum GLTFLoaderError {
  GltfFileLoadError(gltf::Error),
  UnsupportedGLTFExtension(String),
}

/// the root of the gltf will be loaded under the target node
pub fn load_gltf(
  path: impl AsRef<Path>,
  target: EntityHandle<SceneNodeEntity>,
  writer: &mut SceneWriter,
) -> Result<GltfLoadResult, GLTFLoaderError> {
  let parse_result = parse_gltf(path)?;
  let result = write_gltf_at_node(target, writer, parse_result);
  Ok(result)
}

/// this call should be spawned to work thread
pub fn parse_gltf(path: impl AsRef<Path>) -> Result<GltfParseResult, GLTFLoaderError> {
  let path = path.as_ref().to_path_buf();
  let (document, buffers, images) =
    gltf::import(&path).map_err(GLTFLoaderError::GltfFileLoadError)?;

  for ext in document.extensions_required() {
    if !SUPPORTED_GLTF_EXTENSIONS.contains(&ext) {
      return Err(GLTFLoaderError::UnsupportedGLTFExtension(ext.to_string()));
    }
  }

  Ok(GltfParseResult {
    path: Some(path),
    document,
    buffers,
    images,
  })
}

/// this call should be spawned to work thread
pub fn parse_gltf_from_buffer(buffer: &[u8]) -> Result<GltfParseResult, GLTFLoaderError> {
  let (document, buffers, images) =
    gltf::import_slice(buffer).map_err(GLTFLoaderError::GltfFileLoadError)?;

  for ext in document.extensions_required() {
    if !SUPPORTED_GLTF_EXTENSIONS.contains(&ext) {
      return Err(GLTFLoaderError::UnsupportedGLTFExtension(ext.to_string()));
    }
  }

  Ok(GltfParseResult {
    path: None,
    document,
    buffers,
    images,
  })
}

pub struct GltfParseResult {
  path: Option<PathBuf>,
  document: gltf::Document,
  buffers: Vec<gltf::buffer::Data>,
  images: Vec<gltf::image::Data>,
}

pub fn write_gltf_at_node(
  target: EntityHandle<SceneNodeEntity>,
  writer: &mut SceneWriter,
  gltf: GltfParseResult,
) -> GltfLoadResult {
  let GltfParseResult {
    document,
    mut buffers,
    images,
    path,
  } = gltf;

  let mut ctx = Context {
    images,
    attributes: buffers
      .drain(..)
      .map(|buffer| ExternalRefPtr::new(buffer.0))
      .collect(),
    result: Default::default(),
    io: writer,
  };

  ctx.result.path = path;

  for ext in document.extensions_used() {
    if !SUPPORTED_GLTF_EXTENSIONS.contains(&ext) {
      ctx
        .result
        .used_but_not_supported_extensions
        .push(ext.to_string());
    }
  }

  let node_count = document.nodes().len();
  ctx.io.node_writer.notify_reserve_changes(node_count);
  ctx.result.node_map.grow_to(node_count);
  let mut model_count = 0;
  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(target, &node, &mut ctx, &mut model_count);
    }
  }

  let skin_count = document.skins().len();
  ctx.io.skin_writer.notify_reserve_changes(skin_count);
  ctx.result.skin_map.grow_to(skin_count);
  for skin in document.skins() {
    build_skin(skin, &mut ctx);
  }

  let animation_count = document.animations().len();
  ctx.io.animation.notify_reserve_changes(animation_count);
  ctx.result.animation_map.grow_to(animation_count);
  for animation in document.animations() {
    build_animation(animation, &mut ctx);
  }

  let image_count = ctx.images.len();
  ctx.io.tex_writer.notify_reserve_changes(image_count);
  ctx.io.sampler_writer.notify_reserve_changes(image_count);

  ctx.result.scene_models.reserve(model_count);
  ctx.result.standard_models.reserve(model_count);
  ctx.io.model_writer.notify_reserve_changes(model_count);
  ctx.io.std_model_writer.notify_reserve_changes(model_count);
  ctx
    .io
    .mesh_writer
    .notify_reserve_changes(model_count, &mut ctx.io.buffer_writer);

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_content_recursive(&node, &mut ctx);
    }
  }

  ctx.result
}

struct Context<'a> {
  io: &'a mut SceneWriter,
  images: Vec<gltf::image::Data>,
  attributes: Vec<ExternalRefPtr<Vec<u8>>>,
  result: GltfLoadResult,
}

#[derive(Default)]
pub struct GltfLoadResult {
  pub path: Option<PathBuf>,
  pub node_map: IndexKeptVec<EntityHandle<SceneNodeEntity>>,
  pub view_map: IndexKeptVec<UnTypedBufferView>,
  pub skin_map: IndexKeptVec<EntityHandle<SceneSkinEntity>>,
  pub joints: Vec<EntityHandle<SceneJointEntity>>,
  pub animation_map: IndexKeptVec<EntityHandle<SceneAnimationEntity>>,
  pub animation_channels: Vec<AnimationChannelEntities>,
  pub directional_light_map: IndexKeptVec<EntityHandle<DirectionalLightEntity>>,
  pub point_light_map: IndexKeptVec<EntityHandle<PointLightEntity>>,
  pub spot_light_map: IndexKeptVec<EntityHandle<SpotLightEntity>>,
  pub used_but_not_supported_extensions: Vec<String>,
  pub scene_models: Vec<EntityHandle<SceneModelEntity>>,
  pub standard_models: Vec<EntityHandle<StandardModelEntity>>,
  pub materials: IndexKeptVec<SceneMaterialDataView>,
  // key: (index of mesh in gltf doc, index of primitive in gltf mesh)
  pub meshes: Vec<AttributesMeshEntities>,
  /// map (image id, srgbness) => created texture
  pub images: FastHashMap<(usize, bool), EntityHandle<SceneTexture2dEntity>>,
  pub samplers: IndexKeptVec<EntityHandle<SceneSamplerEntity>>,
  pub new_created_skeleton_root: Vec<EntityHandle<SceneNodeEntity>>,
}

fn write_label<E: EntitySemantic>(
  writer: &mut EntityWriter<E>,
  id: EntityHandle<E>,
  label: Option<&str>,
) {
  if let Some(name) = label {
    writer.write::<LabelOf<E>>(id, name.to_owned());
  }
}

/// https://docs.rs/gltf/latest/gltf/struct.Node.html
fn create_node_recursive(
  parent_to_attach: EntityHandle<SceneNodeEntity>,
  gltf_node: &Node,
  ctx: &mut Context,
  model_count: &mut usize,
) {
  let node = ctx.io.create_child(parent_to_attach);
  ctx.result.node_map.insert(gltf_node.index(), node);

  ctx
    .io
    .set_local_matrix(node, map_transform(gltf_node.transform()));
  write_label(&mut ctx.io.node_writer, node, gltf_node.name());

  if gltf_node.mesh().is_some() {
    *model_count += 1;
  }

  for gltf_node in gltf_node.children() {
    create_node_recursive(node, &gltf_node, ctx, model_count)
  }
}

fn create_node_content_recursive(gltf_node: &Node, ctx: &mut Context) {
  let node = *ctx.result.node_map.get(gltf_node.index());

  if let Some(mesh) = gltf_node.mesh() {
    for primitive in mesh.primitives() {
      build_model(node, primitive, gltf_node, ctx, mesh.name(), mesh.index());
    }
  }

  if let Some(light) = gltf_node.light() {
    let intensity = light.intensity();
    let color = light.color();
    let intensity = Vec3::from(color) * intensity;
    let cutoff_distance = light.range().unwrap_or(DEFAULT_CUTOFF_DISTANCE);
    let scene = ctx.io.scene;
    match light.kind() {
      gltf::khr_lights_punctual::Kind::Directional => {
        let scene_light = DirectionalLightDataView {
          illuminance: intensity,
          node,
          scene,
        }
        .write(&mut ctx.io.directional_light_writer);
        ctx
          .result
          .directional_light_map
          .insert(light.index(), scene_light);
      }
      gltf::khr_lights_punctual::Kind::Point => {
        let scene_light = PointLightDataView {
          intensity,
          cutoff_distance,
          node,
          scene,
        }
        .write(&mut ctx.io.point_light_writer);
        ctx
          .result
          .point_light_map
          .insert(light.index(), scene_light);
      }
      gltf::khr_lights_punctual::Kind::Spot {
        inner_cone_angle,
        outer_cone_angle,
      } => {
        let scene_light = SpotLightDataView {
          intensity,
          cutoff_distance,
          half_cone_angle: outer_cone_angle,
          half_penumbra_angle: inner_cone_angle,
          node,
          scene,
        }
        .write(&mut ctx.io.spot_light_writer);
        ctx.result.spot_light_map.insert(light.index(), scene_light);
      }
    }
  }

  for gltf_node in gltf_node.children() {
    create_node_content_recursive(&gltf_node, ctx)
  }
}

/// note, here we not share the std model for the same gltf mesh
///
/// this can be improved, but it's tricky because the gltf mesh skin is decided by the node(scene model level)
/// but the rendiation's skin is in standard model level
fn build_model(
  node: EntityHandle<SceneNodeEntity>,
  primitive: gltf::Primitive,
  gltf_node: &gltf::Node,
  ctx: &mut Context,
  name: Option<&str>,
  idx: usize,
) -> EntityHandle<SceneModelEntity> {
  let attributes = primitive
    .attributes()
    .map(|(semantic, accessor)| {
      let semantic = map_attribute_semantic(semantic);
      let mut att = build_accessor(accessor, ctx);
      // expand joint indices from u8/u16 to u32
      if let AttributeSemantic::Joints(_) = &semantic {
        let read = att.read();
        if att.item_byte_size == 4 {
          let indices = read.visit_slice::<Vec4<u8>>().unwrap();
          let new_indices = indices
            .iter()
            .map(|v| v.map(|v| v as u32))
            .collect::<Vec<_>>();
          att = AttributeAccessor::create_owned(new_indices, 4 * 4)
        } else if att.item_byte_size == 2 * 4 {
          let indices = read.visit_slice::<Vec4<u16>>().unwrap();
          let new_indices = indices
            .iter()
            .map(|v| v.map(|v| v as u32))
            .collect::<Vec<_>>();
          att = AttributeAccessor::create_owned(new_indices, 4 * 4)
        } else {
          panic!(
            "joint indices must be vec4<u8> or vec4<u16>, item_byte_size: {}",
            att.item_byte_size
          )
        }
      }

      if let AttributeSemantic::Weights(_) = &semantic {
        if att.item_byte_size != 16 {
          panic!(
            "current implementation only supports vec4<f32>, item_byte_size: {}",
            att.item_byte_size
          )
        }
      }

      (semantic, att)
    })
    .collect();

  let indices = primitive.indices().map(|indices| {
    let format = match indices.data_type() {
      gltf::accessor::DataType::U16 => AttributeIndexFormat::Uint16,
      gltf::accessor::DataType::U32 => AttributeIndexFormat::Uint32,
      _ => unreachable!(),
    };
    (format, build_accessor(indices, ctx))
  });

  let mode = map_draw_mode(primitive.mode()).unwrap();

  let mesh = AttributesMesh {
    attributes,
    indices,
    mode,
  };
  let mesh = ctx.io.write_attribute_mesh(mesh);

  let material = build_material(primitive.material(), ctx);

  let mut model = StandardModelDataView {
    material,
    mesh: mesh.mesh,
    skin: None,
  };

  ctx.result.meshes.push(mesh);

  if let Some(skin) = gltf_node.skin() {
    let sk = ctx.result.skin_map.get(skin.index());
    model.skin = Some(*sk)
  }

  let model = model.write(&mut ctx.io.std_model_writer);
  ctx.result.standard_models.push(model);

  let sm = SceneModelDataView {
    model,
    scene: ctx.io.scene,
    node,
  };

  let sm = sm.write(&mut ctx.io.model_writer);
  let name = name.map(|n| format!("{}-{}", n, idx));
  write_label(&mut ctx.io.model_writer, sm, name.as_deref());

  ctx.result.scene_models.push(sm);

  sm
}

fn build_animation(animation: gltf::Animation, ctx: &mut Context) {
  let animation_handle = ctx
    .io
    .animation
    .new_entity(|w| w.write::<SceneAnimationBelongsToScene>(&ctx.io.scene.some_handle()));

  write_label(&mut ctx.io.animation, animation_handle, animation.name());

  animation.channels().for_each(|channel| {
    let target = channel.target();
    let node = *ctx.result.node_map.get(target.node().index());

    let field = map_animation_field(target.property());
    let gltf_sampler = channel.sampler();
    let sampler = AnimationSampler {
      interpolation: map_animation_interpolation(gltf_sampler.interpolation()),
      field,
      input: build_accessor(gltf_sampler.input(), ctx),
      output: build_accessor(gltf_sampler.output(), ctx),
    };

    let channel = AnimationChannelDataView {
      sampler,
      target_node: node,
      animation: animation_handle,
    };

    let channel = channel.write(ctx.io);

    ctx.result.animation_channels.push(channel);
  });

  ctx
    .result
    .animation_map
    .insert(animation.index(), animation_handle);
}

fn build_skin(skin: gltf::Skin, ctx: &mut Context) {
  // https://stackoverflow.com/questions/64734695/what-does-it-mean-when-gltf-does-not-specify-a-skeleton-value-in-a-skin
  let skeleton_root = skin
    .skeleton()
    .and_then(|n| ctx.result.node_map.try_get(n.index()))
    .copied()
    .unwrap_or_else(|| {
      let new = ctx.io.create_root_child();
      ctx.result.new_created_skeleton_root.push(new);
      new
    });

  let skin_handle = ctx
    .io
    .skin_writer
    .new_entity(|w| w.write::<SceneSkinRoot>(&skeleton_root.some_handle()));

  ctx.result.skin_map.insert(skin.index(), skin_handle);

  let matrix_list = if let Some(matrix_list) = skin.inverse_bind_matrices() {
    let matrix_list = build_accessor(matrix_list, ctx);
    let matrix_list = matrix_list.read();
    matrix_list
      .visit_slice::<Mat4<f32>>()
      .unwrap()
      .to_vec()
      .into()
  } else {
    None
  };

  for (i, joint) in skin.joints().enumerate() {
    let node = *ctx.result.node_map.get(joint.index());

    let mat = if let Some(matrix_list) = matrix_list.as_ref() {
      matrix_list[i]
    } else {
      Mat4::identity()
    };

    let joint = ctx.io.joint_writer.new_entity(|w| {
      w.write::<SceneJointBelongToSkin>(&skin_handle.some_handle())
        .write::<SceneJointRefNode>(&node.some_handle())
        .write::<SceneJointInverseBindMatrix>(&mat)
        .write::<SceneJointSkinIndex>(&(i as u32))
    });
    ctx.result.joints.push(joint);
  }
}

fn build_data_view(view: gltf::buffer::View, ctx: &mut Context) -> UnTypedBufferView {
  let buffers = &ctx.attributes;
  ctx
    .result
    .view_map
    .get_insert_with(view.index(), || {
      let buffer = buffers[view.buffer().index()].clone();
      UnTypedBufferView {
        buffer: buffer.ptr.clone(),
        range: BufferViewRange {
          offset: view.offset() as u64,
          size: NonZeroU64::new(view.length() as u64),
        },
      }
    })
    .clone()
}

fn build_accessor(accessor: gltf::Accessor, ctx: &mut Context) -> AttributeAccessor {
  let view = accessor.view().unwrap(); // not support sparse accessor
  let view = build_data_view(view, ctx);

  let ty = accessor.data_type();
  let dimension = accessor.dimensions();

  let byte_offset = accessor.offset();
  let count = accessor.count();

  let item_byte_size = match ty {
    gltf::accessor::DataType::I8 => 1,
    gltf::accessor::DataType::U8 => 1,
    gltf::accessor::DataType::I16 => 2,
    gltf::accessor::DataType::U16 => 2,
    gltf::accessor::DataType::U32 => 4,
    gltf::accessor::DataType::F32 => 4,
  } * match dimension {
    gltf::accessor::Dimensions::Scalar => 1,
    gltf::accessor::Dimensions::Vec2 => 2,
    gltf::accessor::Dimensions::Vec3 => 3,
    gltf::accessor::Dimensions::Vec4 => 4,
    gltf::accessor::Dimensions::Mat2 => 4,
    gltf::accessor::Dimensions::Mat3 => 9,
    gltf::accessor::Dimensions::Mat4 => 16,
  };

  AttributeAccessor {
    view,
    count,
    byte_offset,
    item_byte_size,
  }
}

fn build_material(material: gltf::Material, ctx: &mut Context) -> SceneMaterialDataView {
  let idx = material.index().unwrap_or(0) + 1; // keep 0 for default material;
  if let Some(re) = ctx.result.materials.try_get(idx).copied() {
    re
  } else {
    let re = build_material_internal(material, ctx);
    ctx.result.materials.insert(idx, re);
    re
  }
}

/// https://docs.rs/gltf/latest/gltf/struct.Material.html
fn build_material_internal(material: gltf::Material, ctx: &mut Context) -> SceneMaterialDataView {
  let pbr = material.pbr_metallic_roughness();

  let alpha_mode = map_alpha(material.alpha_mode());
  let alpha_cut = material.alpha_cutoff().unwrap_or(0.5);

  let emissive_texture = material
    .emissive_texture()
    .map(|tex| build_texture(tex.texture(), true, ctx));

  let normal_texture = material.normal_texture().map(|tex| NormalMappingDataView {
    content: build_texture(tex.texture(), false, ctx),
    scale: tex.scale(),
  });

  if material.unlit() {
    let color_and_alpha = Vec4::from(pbr.base_color_factor());
    let base_color_texture = pbr
      .base_color_texture()
      .map(|tex| build_texture(tex.texture(), true, ctx));
    let mat = UnlitMaterialDataView {
      color: color_and_alpha,
      color_alpha_tex: base_color_texture,
      alpha: AlphaConfigDataView {
        alpha_mode,
        alpha_cutoff: alpha_cut,
        alpha: color_and_alpha.a(),
      },
    }
    .write(&mut ctx.io.unlit_mat_writer);

    write_label(&mut ctx.io.unlit_mat_writer, mat, material.name());
    return SceneMaterialDataView::UnlitMaterial(mat);
  }

  if let Some(pbr_specular) = material.pbr_specular_glossiness() {
    let albedo_texture = pbr_specular
      .diffuse_texture()
      .map(|tex| build_texture(tex.texture(), true, ctx));

    let specular_glossiness_texture = pbr_specular
      .specular_glossiness_texture()
      .map(|tex| build_texture(tex.texture(), true, ctx));

    let albedo_and_alpha = Vec4::from(pbr_specular.diffuse_factor());

    let result = PhysicalSpecularGlossinessMaterialDataView {
      albedo: albedo_and_alpha.xyz(),
      specular: Vec3::from(pbr_specular.specular_factor()),
      glossiness: pbr_specular.glossiness_factor(),
      emissive: Vec3::from(material.emissive_factor()),
      alpha: AlphaConfigDataView {
        alpha_mode,
        alpha_cutoff: alpha_cut,
        alpha: albedo_and_alpha.a(),
      },
      albedo_texture,
      specular_glossiness_texture,
      emissive_texture,
      normal_texture,
    };

    if material.double_sided() {
      // result.states.cull_mode = None;
    }
    let mat = result.write(&mut ctx.io.pbr_sg_mat_writer);
    write_label(&mut ctx.io.pbr_sg_mat_writer, mat, material.name());
    SceneMaterialDataView::PbrSGMaterial(mat)
  } else {
    let base_color_texture = pbr
      .base_color_texture()
      .map(|tex| build_texture(tex.texture(), true, ctx));

    let metallic_roughness_texture = pbr
      .metallic_roughness_texture()
      .map(|tex| build_texture(tex.texture(), false, ctx));

    let color_and_alpha = Vec4::from(pbr.base_color_factor());

    let result = PhysicalMetallicRoughnessMaterialDataView {
      base_color: color_and_alpha.rgb(),
      alpha: AlphaConfigDataView {
        alpha_mode,
        alpha_cutoff: alpha_cut,
        alpha: color_and_alpha.a(),
      },
      roughness: pbr.roughness_factor(),
      metallic: pbr.metallic_factor(),
      emissive: Vec3::from(material.emissive_factor()),
      base_color_texture,
      metallic_roughness_texture,
      emissive_texture,
      normal_texture,
      // reflectance: 0.5, // todo from gltf ior extension
    };

    if material.double_sided() {
      // result.states.cull_mode = None;
    }
    let mat = result.write(&mut ctx.io.pbr_mr_mat_writer);
    write_label(&mut ctx.io.pbr_mr_mat_writer, mat, material.name());
    SceneMaterialDataView::PbrMRMaterial(mat)
  }
}

// i assume all gpu use little endian?
const F16_BYTES: [u8; 2] = half::f16::from_f32_const(1.0).to_le_bytes();
const F32_BYTES: [u8; 4] = 1.0_f32.to_le_bytes();

fn build_image(
  io: &mut SceneWriter,
  data_input: gltf::image::Data,
  require_srgb: bool,
) -> EntityHandle<SceneTexture2dEntity> {
  let mut format = match data_input.format {
    gltf::image::Format::R8 => TextureFormat::R8Unorm,
    gltf::image::Format::R8G8 => TextureFormat::Rg8Unorm,
    gltf::image::Format::R8G8B8 => TextureFormat::Rgba8Unorm, // padding
    gltf::image::Format::R8G8B8A8 => TextureFormat::Rgba8Unorm,
    gltf::image::Format::R16 => TextureFormat::R16Float,
    gltf::image::Format::R16G16 => TextureFormat::Rg16Float,
    gltf::image::Format::R16G16B16 => TextureFormat::Rgba16Float, // padding
    gltf::image::Format::R16G16B16A16 => TextureFormat::Rgba16Float,
    gltf::image::Format::R32G32B32FLOAT => TextureFormat::Rgba32Float, // padding
    gltf::image::Format::R32G32B32A32FLOAT => TextureFormat::Rgba32Float,
  };

  if require_srgb {
    format = format.add_srgb_suffix();
  }

  let data = if let Some((read_bytes, pad_bytes)) = match data_input.format {
    gltf::image::Format::R8G8B8 => (3, [255].as_slice()).into(),
    gltf::image::Format::R16G16B16 => (3 * 2, F16_BYTES.as_slice()).into(),
    gltf::image::Format::R32G32B32FLOAT => (3 * 2, F32_BYTES.as_slice()).into(),
    _ => None,
  } {
    create_padding_buffer(&data_input.pixels, read_bytes, pad_bytes)
  } else {
    data_input.pixels
  };

  let size =
    rendiation_texture_core::Size::from_u32_pair_min_one((data_input.width, data_input.height));

  let image = ExternalRefPtr::new(GPUBufferImage { data, format, size });
  io.tex_writer
    .new_entity(|w| w.write::<SceneTexture2dEntityDirectContent>(&image.into()))
}

fn build_texture(
  texture: gltf::texture::Texture,
  require_srgb: bool,
  ctx: &mut Context,
) -> Texture2DWithSamplingDataView {
  let sampler = texture.sampler();
  let sampler_idx = sampler.index().unwrap_or(0) + 1; // keep 0 for default sampler
  let sampler = *ctx.result.samplers.get_insert_with(sampler_idx, || {
    ctx
      .io
      .sampler_writer
      .new_entity(|w| w.write::<SceneSamplerInfo>(&map_sampler(sampler)))
  });

  let image_index = texture.source().index();
  let texture = *ctx
    .result
    .images
    .entry((image_index, require_srgb))
    .or_insert_with(|| {
      build_image(
        ctx.io,
        ctx.images.get(image_index).unwrap().clone(),
        require_srgb,
      )
    });

  Texture2DWithSamplingDataView { texture, sampler }
}

impl GltfLoadResult {
  /// note, caller must assure any other entity is not referencing the loaded gltf entity
  pub fn unload(self, writer: &mut SceneWriter) {
    for (_, light) in self.directional_light_map.iter() {
      writer.directional_light_writer.delete_entity(*light);
    }
    for (_, light) in self.point_light_map.iter() {
      writer.point_light_writer.delete_entity(*light);
    }
    for (_, light) in self.spot_light_map.iter() {
      writer.spot_light_writer.delete_entity(*light);
    }
    for joint in self.joints {
      writer.joint_writer.delete_entity(joint);
    }
    for (_, skin) in self.skin_map.iter() {
      writer.skin_writer.delete_entity(*skin);
    }
    for (_, animation) in self.animation_map.iter() {
      writer.animation.delete_entity(*animation);
    }
    for c in self.animation_channels {
      c.delete_entities(writer);
    }
    for (_, node) in self.node_map.iter() {
      writer.node_writer.delete_entity(*node);
    }
    for node in self.new_created_skeleton_root {
      writer.node_writer.delete_entity(node);
    }
    for mesh in self.meshes.iter() {
      mesh.clean_up(&mut writer.mesh_writer, &mut writer.buffer_writer);
    }

    for (_, material) in self.materials.iter() {
      match material {
        SceneMaterialDataView::UnlitMaterial(entity_handle) => {
          writer.unlit_mat_writer.delete_entity(*entity_handle);
        }
        SceneMaterialDataView::PbrSGMaterial(entity_handle) => {
          writer.pbr_sg_mat_writer.delete_entity(*entity_handle);
        }
        SceneMaterialDataView::PbrMRMaterial(entity_handle) => {
          writer.pbr_mr_mat_writer.delete_entity(*entity_handle);
        }
        _ => {}
      }
    }

    for sm in self.scene_models.iter() {
      writer.model_writer.delete_entity(*sm);
    }

    for std_model in self.standard_models.iter() {
      writer.std_model_writer.delete_entity(*std_model);
    }

    for (_, texture) in self.images.iter() {
      writer.tex_writer.delete_entity(*texture);
    }

    for (_, sampler) in self.samplers.iter() {
      writer.sampler_writer.delete_entity(*sampler);
    }
  }
}
