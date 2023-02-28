#![feature(local_key_cell_methods)]

use std::path::Path;
use std::{collections::HashMap, sync::Arc};

use __core::marker::PhantomData;
use __core::num::NonZeroU64;
use gltf::{Node, Result as GltfResult};
use rendiation_algebra::*;
use rendiation_geometry::{CubicBezierSegment, SpaceLineSegment, StraitLine};
use rendiation_scene_core::{
  AttributeAccessor, AttributesMesh, BufferViewRange, GeometryBuffer, GeometryBufferInner,
  IndexFormat, NormalMapping, PhysicalMetallicRoughnessMaterial, Scene, SceneMaterialType,
  SceneMeshType, SceneModel, SceneModelHandle, SceneModelImpl, SceneModelType, SceneNode,
  SceneTexture2D, SceneTexture2DType, StandardModel, Texture2DWithSamplingData,
  TextureWithSamplingData, UnTypedBufferView,
};
use shadergraph::*;
use webgpu::{TextureFormat, WebGPU2DTextureSource};

mod convert_utils;
use convert_utils::*;

pub fn load_gltf(path: impl AsRef<Path>, scene: &Scene) -> GltfResult<GltfLoadResult> {
  let scene_inner = scene.read();
  let (document, mut buffers, mut images) = gltf::import(path)?;

  let mut ctx = Context {
    images: images.drain(..).map(build_image).collect(),
    attributes: buffers
      .drain(..)
      .map(|buffer| GeometryBufferInner { buffer: buffer.0 }.into())
      .collect(),
    result: Default::default(),
  };

  let root = scene_inner.root().clone();

  drop(scene_inner);

  for gltf_scene in document.scenes() {
    for node in gltf_scene.nodes() {
      create_node_recursive(scene, root.clone(), &node, &mut ctx);
    }
  }

  for animation in document.animations() {
    build_animation(animation, &mut ctx);
  }

  Ok(ctx.result)
}

struct Context {
  images: Vec<SceneTexture2D>,
  attributes: Vec<GeometryBuffer>,
  result: GltfLoadResult,
}

#[derive(Default)]
pub struct GltfLoadResult {
  pub primitive_map: HashMap<usize, SceneModelHandle>,
  pub node_map: HashMap<usize, SceneNode>,
  pub view_map: HashMap<usize, UnTypedBufferView>,
  pub animations: Vec<SceneAnimation>,
}

/// https://docs.rs/gltf/latest/gltf/struct.Node.html
fn create_node_recursive(
  scene: &Scene,
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
      let model_handle = scene.insert_model(model);
      ctx.result.primitive_map.insert(index, model_handle);
    }
  }

  for gltf_node in gltf_node.children() {
    create_node_recursive(scene, node.clone(), &gltf_node, ctx)
  }
}

fn build_model(node: &SceneNode, primitive: gltf::Primitive, ctx: &mut Context) -> SceneModel {
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
      gltf::accessor::DataType::U16 => IndexFormat::Uint16,
      gltf::accessor::DataType::U32 => IndexFormat::Uint32,
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
  let mesh = SceneMeshType::AttributesMesh(mesh.into());

  let material = build_pbr_material(primitive.material(), ctx);
  let material = SceneMaterialType::PhysicalMetallicRoughness(material.into());

  let model = StandardModel {
    material,
    mesh,
    group: Default::default(),
  };
  let model = SceneModelType::Standard(model.into());
  let model = SceneModelImpl {
    model,
    node: node.clone(),
  };
  SceneModel::new(model)
}

pub struct SceneAnimation {
  pub channels: Vec<SceneAnimationChannel>,
}

impl SceneAnimation {
  pub fn update(&self, time: f32) {
    self.channels.iter().for_each(|c| c.update(time))
  }
}

/// An animation channel combines an animation sampler with a target property being animated.
pub struct SceneAnimationChannel {
  pub target_node: SceneNode,
  pub target_field: SceneAnimationField,
  pub sampler: AnimationSampler,
}

impl SceneAnimationChannel {
  pub fn update(&self, time: f32) {
    match self.target_field {
      SceneAnimationField::MorphTargetWeights => {
        if let InterpolationItem::Float(item) = self.sampler.sample(time) {
          todo!();
        }
      }
      SceneAnimationField::Position => {
        if let InterpolationItem::Vec3(item) = self.sampler.sample(time) {
          let mut local_mat = self.target_node.get_local_matrix();
          local_mat.set_position(item);
          self.target_node.set_local_matrix(local_mat);
        }
      }
      SceneAnimationField::Rotation => {
        if let InterpolationItem::Quaternion(item) = self.sampler.sample(time) {
          let mut local_mat = self.target_node.get_local_matrix();
          local_mat.set_rotation(item);
          self.target_node.set_local_matrix(local_mat);
        }
      }
      SceneAnimationField::Scale => {
        if let InterpolationItem::Vec3(item) = self.sampler.sample(time) {
          let mut local_mat = self.target_node.get_local_matrix();
          local_mat.set_scale(item);
          self.target_node.set_local_matrix(local_mat);
        }
      }
    }
  }
}

#[derive(Clone)]
pub enum SceneAnimationField {
  MorphTargetWeights,
  Position,
  Rotation,
  Scale,
}

#[derive(Clone)]
pub enum SceneAnimationInterpolation {
  Linear,
  Step,
  Cubic,
}

/// Decide the behavior of the cubic interpolation if the start or end point has not enough data
// pub enum CubicEndingStyle {
//   ZeroSlop,
//   ZeroCurvature,
//   WrapAround,
// }

// type CubicInterpolationLine =  SpaceLineSegment<Vec2<f32>, CubicBezierSegment<Vec2<f32>>>;

// impl CubicEndingStyle {
//     pub fn build_cubic(&self, start: Vec2<f32>, end: Vec2<f32>) -> CubicInterpolationLine {

//     }
// }

trait KeyframeTrack {
  type Value;
  fn sample_animation(&self) -> Self::Value;
}

/// An animation sampler combines timestamps with a sequence of
/// output values and defines an interpolation algorithm.
#[derive(Clone)]
pub struct AnimationSampler {
  pub interpolation: SceneAnimationInterpolation,
  pub input: AttributeAccessor,
  pub output: AttributeAccessor,
}

enum InterpolationItem {
  Vec3(Vec3<f32>),
  Quaternion(Quat<f32>),
  Float(f32),
}

enum InterpolationVertex {
  LinearOrStep(InterpolationItem),
  Cubic {
    enter: InterpolationItem,
    center: InterpolationItem,
    exit: InterpolationItem,
  },
}

impl AnimationSampler {
  pub fn sample(&self, time: f32) -> Option<InterpolationItem> {
    // first, decide which frame interval we are in;
    let index = self
      .input
      .visit_slice::<f32, _>(|slice| {
        // the gltf animation spec doesn't contains start time or loop behavior, we just use abs time
        slice
          .binary_search_by(|v| v.partial_cmp(&time).unwrap_or(core::cmp::Ordering::Equal))
          .ok()
      })
      .flatten()?;

    let (start_time, start_index) = todo!();
    let (end_time, end_index) = todo!();
    let normalized_time = todo!();

    let start = todo!();
    let end = todo!();
    // then we compute a interpolation spline based on interpolation and input output;
    match self.interpolation {
      SceneAnimationInterpolation::Linear => start.lerp(end, normalized_time),
      SceneAnimationInterpolation::Step => {
        if normalized_time == 1. {
          end
        } else {
          start
        }
      }
      SceneAnimationInterpolation::Cubic => {
        //
      }
    }
  }
}

/// this is an optimization, based on the hypnosis that the interpolation spline
/// will be reused in next sample, which avoid the underlayer sampler retrieving
pub struct AnimationSamplerExecutor<I, V> {
  current_time: f32,
  start_time: f32,
  end_time: f32,
  interpolate: Option<I>,
  output: PhantomData<V>,
  sampler: AnimationSampler,
}

impl AnimationSampler {
  // pub fn create_executor<I, V>(&self) -> Option<AnimationSamplerExecutor<I, V>> {
  // }
}

impl<I, V> KeyframeTrack for AnimationSamplerExecutor<I, V> {
  type Value = V;

  fn sample_animation(&self) -> Self::Value {
    todo!()
  }
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
        input: build_accessor(gltf_sampler.input(), ctx),
        output: build_accessor(gltf_sampler.output(), ctx),
      };

      SceneAnimationChannel {
        target_node: node,
        target_field: field,
        sampler,
      }
    })
    .collect();

  ctx.result.animations.push(SceneAnimation { channels })
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
      }
    })
    .clone()
}

fn build_accessor(accessor: gltf::Accessor, ctx: &mut Context) -> AttributeAccessor {
  let view = accessor.view().unwrap(); // not support sparse accessor
  let view = build_data_view(view, ctx);

  let ty = accessor.data_type();
  let dimension = accessor.dimensions();

  let start = accessor.offset();
  let count = accessor.count();

  let item_size = match ty {
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
    start: start / item_size,
    item_size,
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
    .map(|tex| build_texture(tex.texture(), ctx));

  let metallic_roughness_texture = pbr
    .metallic_roughness_texture()
    .map(|tex| build_texture(tex.texture(), ctx));

  let emissive_texture = material
    .emissive_texture()
    .map(|tex| build_texture(tex.texture(), ctx));

  let normal_texture = material.normal_texture().map(|tex| NormalMapping {
    content: build_texture(tex.texture(), ctx),
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

#[derive(Debug, Clone)]
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

fn build_image(data_input: gltf::image::Data) -> SceneTexture2D {
  let format = match data_input.format {
    gltf::image::Format::R8 => TextureFormat::R8Unorm,
    gltf::image::Format::R8G8 => TextureFormat::Rg8Unorm,
    gltf::image::Format::R8G8B8 => TextureFormat::Rgba8UnormSrgb, // padding
    gltf::image::Format::R8G8B8A8 => TextureFormat::Rgba8UnormSrgb,
    gltf::image::Format::B8G8R8 => TextureFormat::Bgra8UnormSrgb, // padding
    gltf::image::Format::B8G8R8A8 => TextureFormat::Bgra8UnormSrgb,
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
  let image: Box<dyn WebGPU2DTextureSource> = Box::new(image);
  let image = SceneTexture2DType::Foreign(Arc::new(image));
  SceneTexture2D::new(image)
}

fn build_texture(texture: gltf::texture::Texture, ctx: &mut Context) -> Texture2DWithSamplingData {
  let sampler = map_sampler(texture.sampler());
  let image_index = texture.source().index();
  let texture = ctx.images.get(image_index).unwrap().clone();
  TextureWithSamplingData { texture, sampler }
}
