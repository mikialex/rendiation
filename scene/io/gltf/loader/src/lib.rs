#![feature(local_key_cell_methods)]

use std::collections::HashMap;
use std::{path::Path, rc::Rc};

use __core::num::NonZeroU64;
use gltf::{Node, Result as GltfResult};
use rendiation_algebra::*;
use rendiation_scene_webgpu::{
  AttributeAccessor, AttributesMesh, GeometryBuffer, IntoStateControl, MeshModel, MeshModelImpl,
  PhysicalMaterial, Scene, SceneModelShareable, SceneNode, SceneTexture2D, StateControl,
  TextureWithSamplingData, TypedBufferView, WebGPUScene,
};
use rendiation_scene_webgpu::{SceneModelHandle, WebGPUSceneExtension};
use shadergraph::*;
use webgpu::{GPUBufferViewRange, TextureFormat, WebGPU2DTextureSource};

mod convert_utils;
use convert_utils::*;

fn build_model(
  node: &SceneNode,
  primitive: gltf::Primitive,
  ctx: &mut Context,
) -> impl SceneModelShareable {
  let attributes = primitive
    .attributes()
    .map(|(semantic, accessor)| {
      (
        map_attribute_semantic(semantic),
        build_geom_buffer(accessor, ctx),
      )
    })
    .collect();

  let indices = primitive.indices().map(|indices| {
    let format = match indices.data_type() {
      gltf::accessor::DataType::U16 => webgpu::IndexFormat::Uint16,
      gltf::accessor::DataType::U32 => webgpu::IndexFormat::Uint32,
      _ => unreachable!(),
    };
    (format, build_geom_buffer(indices, ctx))
  });

  let mode = map_draw_mode(primitive.mode()).unwrap();

  let mesh = AttributesMesh {
    attributes,
    indices,
    mode,
  };

  let material = build_pbr_material(primitive.material(), ctx);

  let model = MeshModelImpl::new(material, mesh, node.clone());
  MeshModel::new(model)
}

fn build_data_view(view: gltf::buffer::View, ctx: &mut Context) -> TypedBufferView {
  let buffers = &ctx.attributes;
  ctx
    .result
    .view_map
    .entry(view.index())
    .or_insert_with(|| {
      let buffer = buffers[view.buffer().index()].clone();
      TypedBufferView {
        buffer,
        range: GPUBufferViewRange {
          offset: view.offset() as u64,
          size: NonZeroU64::new(view.length() as u64),
        },
      }
    })
    .clone()
}

fn build_geom_buffer(accessor: gltf::Accessor, ctx: &mut Context) -> AttributeAccessor {
  let view = accessor.view().unwrap(); // not support sparse accessor
  let view = build_data_view(view, ctx);

  let ty = accessor.data_type();
  let dimension = accessor.dimensions();

  let start = accessor.offset();
  let count = accessor.count();

  let stride = match ty {
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
    start: start / stride,
    stride,
  }
}

pub fn load_gltf_test(
  path: impl AsRef<Path>,
  scene: &mut Scene<WebGPUScene>,
) -> GltfResult<GltfLoadResult> {
  let (document, mut buffers, mut images) = gltf::import(path)?;

  let mut ctx = Context {
    images: images.drain(..).map(build_image).collect(),
    attributes: buffers
      .drain(..)
      .map(|buffer| Rc::new(GeometryBuffer::new(buffer.0)))
      .collect(),
    result: Default::default(),
  };

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(scene, scene.root().clone(), &node, &mut ctx);
    }
  }
  Ok(ctx.result)
}

struct Context {
  images: Vec<SceneTexture2D<WebGPUScene>>,
  attributes: Vec<Rc<GeometryBuffer>>,
  result: GltfLoadResult,
}

#[derive(Default)]
pub struct GltfLoadResult {
  pub primitive_map: HashMap<usize, SceneModelHandle>,
  pub node_map: HashMap<usize, SceneNode>,
  pub view_map: HashMap<usize, TypedBufferView>,
}

#[derive(Debug)]
pub struct GltfImage {
  data: Vec<u8>,
  format: webgpu::TextureFormat,
  size: rendiation_texture::Size,
}

impl WebGPU2DTextureSource for GltfImage {
  fn format(&self) -> webgpu::TextureFormat {
    self.format
  }

  fn as_bytes(&self) -> &[u8] {
    self.data.as_slice()
  }

  fn size(&self) -> rendiation_texture::Size {
    self.size
  }
}

fn build_image(data_input: gltf::image::Data) -> SceneTexture2D<WebGPUScene> {
  let format = match data_input.format {
    gltf::image::Format::R8 => TextureFormat::R8Unorm,
    gltf::image::Format::R8G8 => TextureFormat::Rg8Unorm,
    gltf::image::Format::R8G8B8 => TextureFormat::Rgba8Unorm, // padding
    gltf::image::Format::R8G8B8A8 => TextureFormat::Rgba8Unorm,
    gltf::image::Format::B8G8R8 => TextureFormat::Bgra8Unorm, // padding
    gltf::image::Format::B8G8R8A8 => TextureFormat::Bgra8Unorm,
    // todo check the follow format
    gltf::image::Format::R16 => TextureFormat::R16Float,
    gltf::image::Format::R16G16 => TextureFormat::Rg16Float,
    gltf::image::Format::R16G16B16 => TextureFormat::Rgba16Float, // padding
    gltf::image::Format::R16G16B16A16 => TextureFormat::Rgba16Float,
  };

  let data = if let Some((read_bytes, pad_bytes)) = match data_input.format {
    gltf::image::Format::R8G8B8 => (3, &[255]).into(),
    gltf::image::Format::B8G8R8 => (3, &[255]).into(),
    gltf::image::Format::R16G16B16 => todo!(), // todo what's the u8 repr for f16_1.0??
    _ => None,
  } {
    data_input
      .pixels
      .chunks(read_bytes)
      .flat_map(|c| [c, pad_bytes])
      .flatten()
      .copied()
      .collect()
  } else {
    data_input.pixels
  };

  let size = rendiation_texture::Size::from_u32_pair_min_one((data_input.width, data_input.height));

  let image = GltfImage { data, format, size };
  SceneTexture2D::<WebGPUScene>::new(Box::new(image))
}

/// https://docs.rs/gltf/latest/gltf/struct.Node.html
fn create_node_recursive(
  scene: &mut Scene<WebGPUScene>,
  parent_to_attach: SceneNode,
  gltf_node: &Node,
  ctx: &mut Context,
) {
  println!(
    "Node #{} has {} children",
    gltf_node.index(),
    gltf_node.children().count(),
  );

  let node = parent_to_attach.create_child();
  ctx.result.node_map.insert(gltf_node.index(), node.clone());

  node.set_local_matrix(map_transform(gltf_node.transform()));

  if let Some(mesh) = gltf_node.mesh() {
    for primitive in mesh.primitives() {
      let index = primitive.index();
      let model = build_model(&node, primitive, ctx);
      let model_handle = scene.add_model(model);
      ctx.result.primitive_map.insert(index, model_handle);
    }
  }

  for gltf_node in gltf_node.children() {
    create_node_recursive(scene, node.clone(), &gltf_node, ctx)
  }
}

/// https://docs.rs/gltf/latest/gltf/struct.Material.html
fn build_pbr_material(
  material: gltf::Material,
  ctx: &mut Context,
) -> StateControl<PhysicalMaterial<WebGPUScene>> {
  let pbr = material.pbr_metallic_roughness();
  let albedo_texture = pbr.base_color_texture().map(|tex| {
    let texture = tex.texture();
    let sampler = map_sampler(texture.sampler());
    let image_index = texture.source().index();
    let texture = ctx.images.get(image_index).unwrap().clone();
    TextureWithSamplingData { texture, sampler }
  });

  let mut result = PhysicalMaterial {
    albedo: Vec4::from(pbr.base_color_factor()).xyz(),
    albedo_texture,
  }
  .use_state();

  if material.double_sided() {
    result.states.cull_mode = None;
  }
  result
}
