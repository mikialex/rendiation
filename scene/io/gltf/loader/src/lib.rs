#![feature(const_float_bits_conv)]

use core::num::NonZeroU64;
use std::path::Path;

use fast_hash_collection::*;
use gltf::{Node, Result as GltfResult};
use rendiation_algebra::*;
use rendiation_scene_core::{
  AnimationSampler, AttributeAccessor, AttributeIndexFormat, AttributesMesh, BufferViewRange,
  GeometryBuffer, GeometryBufferImpl, IntoIncrementalSignalPtr, Joint, MaterialEnum, MeshEnum,
  ModelEnum, NormalMapping, PhysicalMetallicRoughnessMaterial, Scene, SceneAnimation,
  SceneAnimationChannel, SceneExt, SceneModel, SceneModelHandle, SceneModelImpl, SceneNode,
  SceneTexture2D, SceneTexture2DType, Skeleton, SkeletonImpl, StandardModel,
  Texture2DWithSamplingData, TextureWithSamplingData, UnTypedBufferView,
};

mod convert_utils;
use convert_utils::*;
use rendiation_texture::{create_padding_buffer, GPUBufferImage, TextureFormat};

pub fn load_gltf(path: impl AsRef<Path>, scene: &Scene) -> GltfResult<GltfLoadResult> {
  let root = scene.root();

  let (document, mut buffers, images) = gltf::import(path)?;

  let mut ctx = Context {
    images,
    build_images: Default::default(),
    attributes: buffers
      .drain(..)
      .map(|buffer| GeometryBufferImpl { buffer: buffer.0 }.into())
      .collect(),
    result: Default::default(),
  };

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(root.clone(), &node, &mut ctx);
    }
  }

  for skin in document.skins() {
    build_skin(skin, &mut ctx);
  }

  for animation in document.animations() {
    build_animation(animation, &mut ctx);
  }

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_content_recursive(scene, &node, &mut ctx);
    }
  }

  Ok(ctx.result)
}

struct Context {
  images: Vec<gltf::image::Data>,
  /// map (image id, srgbness) => created texture
  build_images: FastHashMap<(usize, bool), SceneTexture2D>,
  attributes: Vec<GeometryBuffer>,
  result: GltfLoadResult,
}

#[derive(Default)]
pub struct GltfLoadResult {
  pub primitive_map: FastHashMap<usize, SceneModelHandle>,
  pub node_map: FastHashMap<usize, SceneNode>,
  pub view_map: FastHashMap<usize, UnTypedBufferView>,
  pub skin_map: FastHashMap<usize, Skeleton>,
  pub animations: Vec<SceneAnimation>,
}

/// https://docs.rs/gltf/latest/gltf/struct.Node.html
fn create_node_recursive(parent_to_attach: SceneNode, gltf_node: &Node, ctx: &mut Context) {
  println!(
    "Node #{} has {} children",
    gltf_node.index(),
    gltf_node.children().count(),
  );

  let node = parent_to_attach.create_child();
  ctx.result.node_map.insert(gltf_node.index(), node.clone());

  node.set_local_matrix(map_transform(gltf_node.transform()));

  for gltf_node in gltf_node.children() {
    create_node_recursive(node.clone(), &gltf_node, ctx)
  }
}

fn create_node_content_recursive(scene: &Scene, gltf_node: &Node, ctx: &mut Context) {
  let node = ctx.result.node_map.get(&gltf_node.index()).unwrap().clone();

  if let Some(mesh) = gltf_node.mesh() {
    for primitive in mesh.primitives() {
      let index = primitive.index();
      let model = build_model(node.clone(), primitive, gltf_node, ctx);

      let model_handle = scene.insert_model(model);
      ctx.result.primitive_map.insert(index, model_handle);
    }
  }

  for gltf_node in gltf_node.children() {
    create_node_content_recursive(scene, &gltf_node, ctx)
  }
}

fn build_model(
  node: SceneNode,
  primitive: gltf::Primitive,
  gltf_node: &gltf::Node,
  ctx: &mut Context,
) -> SceneModel {
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
  let mesh = MeshEnum::AttributesMesh(mesh.into());

  let material = build_pbr_material(primitive.material(), ctx);
  let material = MaterialEnum::PhysicalMetallicRoughness(material.into());

  let mut model = StandardModel::new(material, mesh);

  if let Some(skin) = gltf_node.skin() {
    let sk = ctx.result.skin_map.get(&skin.index()).unwrap();
    model.skeleton = Some(sk.clone())
  }

  let model = ModelEnum::Standard(model.into());
  let model = SceneModelImpl::new(model, node);
  SceneModel::new(model)
}

fn build_animation(animation: gltf::Animation, ctx: &mut Context) {
  let channels = animation
    .channels()
    .map(|channel| {
      let target = channel.target();
      let node = ctx
        .result
        .node_map
        .get(&target.node().index())
        .unwrap()
        .clone();

      let field = map_animation_field(target.property());
      let gltf_sampler = channel.sampler();
      let sampler = AnimationSampler {
        interpolation: map_animation_interpolation(gltf_sampler.interpolation()),
        field,
        input: build_accessor(gltf_sampler.input(), ctx),
        output: build_accessor(gltf_sampler.output(), ctx),
      };

      SceneAnimationChannel {
        target_node: node,
        sampler,
      }
    })
    .collect();

  ctx.result.animations.push(SceneAnimation { channels })
}

fn build_skin(skin: gltf::Skin, ctx: &mut Context) {
  let mut joints: Vec<_> = skin
    .joints()
    .map(|joint_node| Joint {
      node: ctx
        .result
        .node_map
        .get(&joint_node.index())
        .unwrap()
        .clone(),
      bind_inverse: Mat4::identity(),
    })
    .collect();

  if let Some(matrix_list) = skin.inverse_bind_matrices() {
    let matrix_list = build_accessor(matrix_list, ctx);
    let matrix_list = matrix_list.read();
    let list = matrix_list.visit_slice::<Mat4<f32>>().unwrap();
    list.iter().zip(joints.iter_mut()).for_each(|(mat, joint)| {
      joint.bind_inverse = *mat;
    })
  }

  // https://stackoverflow.com/questions/64734695/what-does-it-mean-when-gltf-does-not-specify-a-skeleton-value-in-a-skin
  // let skeleton_root = skin
  //   .skeleton()
  //   .and_then(|n| ctx.result.node_map.get(&n.index()))
  //   .unwrap_or(scene_inner.root());

  let skeleton = SkeletonImpl { joints }.into_ptr();
  ctx.result.skin_map.insert(skin.index(), skeleton);
}

fn build_data_view(view: gltf::buffer::View, ctx: &mut Context) -> UnTypedBufferView {
  let buffers = &ctx.attributes;
  ctx
    .result
    .view_map
    .entry(view.index())
    .or_insert_with(|| {
      let buffer = buffers[view.buffer().index()].clone();
      UnTypedBufferView {
        buffer,
        range: BufferViewRange {
          offset: view.offset() as u64,
          size: NonZeroU64::new(view.length() as u64),
        },
        // byte_stride: view.stride(),
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
fn build_pbr_material(
  material: gltf::Material,
  ctx: &mut Context,
) -> PhysicalMetallicRoughnessMaterial {
  let pbr = material.pbr_metallic_roughness();

  let base_color_texture = pbr
    .base_color_texture()
    .map(|tex| build_texture(tex.texture(), true, ctx));

  let metallic_roughness_texture = pbr
    .metallic_roughness_texture()
    .map(|tex| build_texture(tex.texture(), false, ctx));

  let emissive_texture = material
    .emissive_texture()
    .map(|tex| build_texture(tex.texture(), true, ctx));

  let normal_texture = material.normal_texture().map(|tex| NormalMapping {
    content: build_texture(tex.texture(), false, ctx),
    scale: tex.scale(),
  });

  let alpha_mode = map_alpha(material.alpha_mode());
  let alpha_cut = material.alpha_cutoff().unwrap_or(1.);

  let color_and_alpha = Vec4::from(pbr.base_color_factor());

  let result = PhysicalMetallicRoughnessMaterial {
    base_color: color_and_alpha.rgb(),
    alpha: color_and_alpha.a(),
    alpha_cutoff: alpha_cut,
    alpha_mode,
    roughness: pbr.roughness_factor(),
    metallic: pbr.metallic_factor(),
    emissive: Vec3::from(material.emissive_factor()),
    base_color_texture,
    metallic_roughness_texture,
    emissive_texture,
    normal_texture,
    reflectance: 0.5, // todo from gltf ior extension
  };

  if material.double_sided() {
    // result.states.cull_mode = None;
  }
  result
}

// i assume all gpu use little endian?
const F16_BYTES: [u8; 2] = half::f16::from_f32_const(1.0).to_le_bytes();
const F32_BYTES: [u8; 4] = 1.0_f32.to_le_bytes();

fn build_image(data_input: gltf::image::Data, require_srgb: bool) -> SceneTexture2D {
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

  let size = rendiation_texture::Size::from_u32_pair_min_one((data_input.width, data_input.height));

  let image = GPUBufferImage { data, format, size };
  let image = SceneTexture2DType::GPUBufferImage(image);
  SceneTexture2D::new(image)
}

fn build_texture(
  texture: gltf::texture::Texture,
  require_srgb: bool,
  ctx: &mut Context,
) -> Texture2DWithSamplingData {
  let sampler = map_sampler(texture.sampler()).into_ptr();
  let image_index = texture.source().index();
  let texture = ctx
    .build_images
    .entry((image_index, require_srgb))
    .or_insert_with(|| build_image(ctx.images.get(image_index).unwrap().clone(), require_srgb))
    .clone();

  TextureWithSamplingData { texture, sampler }
}
