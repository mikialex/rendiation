use std::path::Path;

use gltf::{Node, Result};
use rendiation_algebra::*;
use rendiation_scene_webgpu::{
  IntoStateControl, MeshModel, MeshModelImpl, PhysicalMaterial, Scene, SceneNode, SceneTexture2D,
  StateControl, TextureWithSamplingData, WebGPUScene,
};
use rendiation_texture::{Texture2DBuffer, TextureFormatDecider, TextureSampler};
use webgpu::TextureFormat;

#[derive(Clone)]
struct TypedBuffer<T> {
  pub buffer: std::rc::Rc<Vec<T>>,
}

/// like slice, but owned, ref counted cheap clone
struct TypedBufferView<T> {
  pub buffer: TypedBuffer<T>,
  pub start: usize,
  pub count: usize,
}

pub fn load_gltf_test(path: impl AsRef<Path>, scene: &mut Scene<WebGPUScene>) -> Result<()> {
  let (document, mut buffers, mut images) = gltf::import(path)?;

  let mut ctx = Context {
    images: images.drain(..).map(build_image).collect(),
  };

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(scene.root(), &node, &mut ctx);
    }
  }
  Ok(())
}

struct Context {
  images: Vec<SceneTexture2D<WebGPUScene>>,
}

fn build_image(data: gltf::image::Data) -> SceneTexture2D<WebGPUScene> {
  let format = match data.format {
    gltf::image::Format::R8 => todo!(),
    gltf::image::Format::R8G8 => todo!(),
    gltf::image::Format::R8G8B8 => todo!(),
    gltf::image::Format::R8G8B8A8 => (TextureFormat::Rgba8Unorm),
    gltf::image::Format::B8G8R8 => todo!(),
    gltf::image::Format::B8G8R8A8 => todo!(),
    gltf::image::Format::R16 => todo!(),
    gltf::image::Format::R16G16 => todo!(),
    gltf::image::Format::R16G16B16 => todo!(),
    gltf::image::Format::R16G16B16A16 => todo!(),
  };

  let size = rendiation_texture::Size::from_u32_pair_min_one((data.width, data.height));

  // let container = Texture2DBuffer::from_raw(size, data.pixels);

  todo!()
}

/// https://docs.rs/gltf/latest/gltf/struct.Node.html
fn create_node_recursive(parent_to_attach: &SceneNode, gltf_node: &Node, ctx: &mut Context) {
  println!(
    "Node #{} has {} children",
    gltf_node.index(),
    gltf_node.children().count(),
  );

  let node = parent_to_attach.create_child();

  let local_mat = match gltf_node.transform() {
    gltf::scene::Transform::Matrix { matrix } => {
      Mat4::new_from_colum(matrix[0], matrix[1], matrix[2], matrix[3])
    }
    gltf::scene::Transform::Decomposed {
      translation,
      rotation,
      scale,
    } => {
      Mat4::scale(scale)
        * Mat4::from(Quat::new(
          rotation[0],
          rotation[1],
          rotation[2],
          rotation[3],
        ))
        * Mat4::translate(translation)
    }
  };
  node.set_local_matrix(local_mat);

  if let Some(mesh) = gltf_node.mesh() {
    for primitive in mesh.primitives() {
      let material = build_pbr_material(primitive.material(), ctx);
      //
      // let model: MeshModel<_, _> = MeshModelImpl::new(material, mesh, node).into();
    }
  }

  for gltf_node in gltf_node.children() {
    create_node_recursive(&node, &gltf_node, ctx)
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

fn map_sampler(sampler: gltf::texture::Sampler) -> rendiation_texture::TextureSampler {
  rendiation_texture::TextureSampler {
    address_mode_u: map_wrapping(sampler.wrap_s()),
    address_mode_v: map_wrapping(sampler.wrap_t()),
    address_mode_w: rendiation_texture::AddressMode::ClampToEdge,
    mag_filter: sampler
      .mag_filter()
      .map(map_mag_filter)
      .unwrap_or(rendiation_texture::FilterMode::Nearest),
    min_filter: sampler
      .min_filter()
      .map(map_min_filter)
      .unwrap_or(rendiation_texture::FilterMode::Nearest),
    mipmap_filter: sampler
      .min_filter()
      .map(map_min_filter_mipmap)
      .unwrap_or(rendiation_texture::FilterMode::Nearest),
  }
}

fn map_wrapping(mode: gltf::texture::WrappingMode) -> rendiation_texture::AddressMode {
  match mode {
    gltf::texture::WrappingMode::ClampToEdge => rendiation_texture::AddressMode::ClampToEdge,
    gltf::texture::WrappingMode::MirroredRepeat => rendiation_texture::AddressMode::MirrorRepeat,
    gltf::texture::WrappingMode::Repeat => rendiation_texture::AddressMode::Repeat,
  }
}

fn map_min_filter(min: gltf::texture::MinFilter) -> rendiation_texture::FilterMode {
  match min {
    gltf::texture::MinFilter::Nearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::Linear => rendiation_texture::FilterMode::Linear,
    gltf::texture::MinFilter::NearestMipmapNearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::LinearMipmapNearest => rendiation_texture::FilterMode::Linear,
    gltf::texture::MinFilter::NearestMipmapLinear => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::LinearMipmapLinear => rendiation_texture::FilterMode::Linear,
  }
}

/// https://www.khronos.org/opengl/wiki/Sampler_Object
fn map_min_filter_mipmap(min: gltf::texture::MinFilter) -> rendiation_texture::FilterMode {
  match min {
    gltf::texture::MinFilter::Nearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::Linear => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::NearestMipmapNearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::LinearMipmapNearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::NearestMipmapLinear => rendiation_texture::FilterMode::Linear,
    gltf::texture::MinFilter::LinearMipmapLinear => rendiation_texture::FilterMode::Linear,
  }
}

fn map_mag_filter(f: gltf::texture::MagFilter) -> rendiation_texture::FilterMode {
  match f {
    gltf::texture::MagFilter::Nearest => todo!(),
    gltf::texture::MagFilter::Linear => todo!(),
  }
}
