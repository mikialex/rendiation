use core::num::NonZeroU64;
use std::path::Path;

use database::*;
use fast_hash_collection::*;
use gltf::Node;
use rendiation_algebra::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
mod convert_utils;
use convert_utils::*;
use rendiation_texture_core::*;

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
  let (document, mut buffers, images) =
    gltf::import(path).map_err(GLTFLoaderError::GltfFileLoadError)?;

  for ext in document.extensions_required() {
    if !SUPPORTED_GLTF_EXTENSIONS.contains(&ext) {
      return Err(GLTFLoaderError::UnsupportedGLTFExtension(ext.to_string()));
    }
  }

  let mut ctx = Context {
    images,
    build_images: Default::default(),
    attributes: buffers
      .drain(..)
      .map(|buffer| ExternalRefPtr::new(buffer.0))
      .collect(),
    result: Default::default(),
    io: writer,
  };

  for ext in document.extensions_used() {
    if !SUPPORTED_GLTF_EXTENSIONS.contains(&ext) {
      ctx
        .result
        .used_but_not_supported_extensions
        .push(ext.to_string());
    }
  }

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(target, &node, &mut ctx);
    }
  }

  //   for skin in document.skins() {
  //     build_skin(skin, &mut ctx);
  //   }

  for animation in document.animations() {
    build_animation(animation, &mut ctx);
  }

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_content_recursive(&node, &mut ctx);
    }
  }

  Ok(ctx.result)
}

struct Context<'a> {
  io: &'a mut SceneWriter,
  images: Vec<gltf::image::Data>,
  /// map (image id, srgbness) => created texture
  build_images: FastHashMap<(usize, bool), EntityHandle<SceneTexture2dEntity>>,
  attributes: Vec<ExternalRefPtr<Vec<u8>>>,
  result: GltfLoadResult,
}

#[derive(Default)]
pub struct GltfLoadResult {
  pub primitive_map: FastHashMap<usize, EntityHandle<SceneModelEntity>>,
  pub node_map: FastHashMap<usize, EntityHandle<SceneNodeEntity>>,
  pub view_map: FastHashMap<usize, UnTypedBufferView>,
  // pub skin_map: FastHashMap<usize, EntityHandle<SceneSkinEntity>>,
  pub animation_map: FastHashMap<usize, EntityHandle<SceneAnimationEntity>>,
  pub directional_light_map: FastHashMap<usize, EntityHandle<DirectionalLightEntity>>,
  pub point_light_map: FastHashMap<usize, EntityHandle<PointLightEntity>>,
  pub spot_light_map: FastHashMap<usize, EntityHandle<SpotLightEntity>>,
  pub used_but_not_supported_extensions: Vec<String>,
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
) {
  println!(
    "Node #{} has {} children",
    gltf_node.index(),
    gltf_node.children().count(),
  );

  let node = ctx.io.create_child(parent_to_attach);
  ctx.result.node_map.insert(gltf_node.index(), node);

  ctx
    .io
    .set_local_matrix(node, map_transform(gltf_node.transform()));
  write_label(&mut ctx.io.node_writer, node, gltf_node.name());

  for gltf_node in gltf_node.children() {
    create_node_recursive(node, &gltf_node, ctx)
  }
}

fn create_node_content_recursive(gltf_node: &Node, ctx: &mut Context) {
  let node = *ctx.result.node_map.get(&gltf_node.index()).unwrap();

  if let Some(mesh) = gltf_node.mesh() {
    for (idx, primitive) in mesh.primitives().enumerate() {
      let index = primitive.index();
      let model_handle = build_model(node, primitive, gltf_node, ctx, mesh.name(), idx);

      ctx.result.primitive_map.insert(index, model_handle);
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
      (
        map_attribute_semantic(semantic),
        build_accessor(accessor, ctx),
      )
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
    groups: Default::default(),
  };
  let mesh = ctx.io.write_attribute_mesh(mesh).mesh;

  let material = build_material(primitive.material(), ctx);

  let model = StandardModelDataView {
    material,
    mesh,
    skin: None,
  };

  if let Some(_skin) = gltf_node.skin() {
    // let sk = ctx.result.skin_map.get(&skin.index()).unwrap();
    // model.skeleton = Some(sk.clone())
  }

  let sm = SceneModelDataView {
    model: model.write(&mut ctx.io.std_model_writer),
    scene: ctx.io.scene,
    node,
  };

  let sm = sm.write(&mut ctx.io.model_writer);
  let name = name.map(|n| format!("{}-{}", n, idx));
  write_label(&mut ctx.io.model_writer, sm, name.as_deref());

  sm
}

fn build_animation(animation: gltf::Animation, ctx: &mut Context) {
  let animation_handle = ctx
    .io
    .animation
    .component_value_writer::<SceneAnimationBelongsToScene>(ctx.io.scene.some_handle())
    .new_entity();

  write_label(&mut ctx.io.animation, animation_handle, animation.name());

  animation.channels().for_each(|channel| {
    let target = channel.target();
    let node = *ctx.result.node_map.get(&target.node().index()).unwrap();

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

    channel.write(ctx.io);
  });

  ctx
    .result
    .animation_map
    .insert(animation.index(), animation_handle);
}

// fn build_skin(skin: gltf::Skin, ctx: &mut Context) {
//   let mut joints: Vec<_> = skin
//     .joints()
//     .map(|joint_node| Joint {
//       node: ctx
//         .result
//         .node_map
//         .get(&joint_node.index())
//         .unwrap()
//         .clone(),
//       bind_inverse: Mat4::identity(),
//     })
//     .collect();

//   if let Some(matrix_list) = skin.inverse_bind_matrices() {
//     let matrix_list = build_accessor(matrix_list, ctx);
//     let matrix_list = matrix_list.read();
//     let list = matrix_list.visit_slice::<Mat4<f32>>().unwrap();
//     list.iter().zip(joints.iter_mut()).for_each(|(mat, joint)| {
//       joint.bind_inverse = *mat;
//     })
//   }

//   // https://stackoverflow.com/questions/64734695/what-does-it-mean-when-gltf-does-not-specify-a-skeleton-value-in-a-skin
//   let skeleton_root = skin
//     .skeleton()
//     .and_then(|n| ctx.result.node_map.get(&n.index()))
//     .unwrap_or(scene_inner.root()); // todo create a new root

//   // let skeleton = SkeletonImpl { joints }.into_ptr();
//   // ctx.result.skin_map.insert(skin.index(), skeleton);
// }

fn build_data_view(view: gltf::buffer::View, ctx: &mut Context) -> UnTypedBufferView {
  let buffers = &ctx.attributes;
  ctx
    .result
    .view_map
    .entry(view.index())
    .or_insert_with(|| {
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

/// https://docs.rs/gltf/latest/gltf/struct.Material.html
fn build_material(material: gltf::Material, ctx: &mut Context) -> SceneMaterialDataView {
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
    .component_value_writer::<SceneTexture2dEntityDirectContent>(image.into())
    .new_entity()
}

fn build_texture(
  texture: gltf::texture::Texture,
  require_srgb: bool,
  ctx: &mut Context,
) -> Texture2DWithSamplingDataView {
  let sampler = ctx
    .io
    .sampler_writer
    .component_value_writer::<SceneSamplerInfo>(map_sampler(texture.sampler()))
    .new_entity();

  let image_index = texture.source().index();
  let texture = *ctx
    .build_images
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
