#![feature(local_key_cell_methods)]

use std::collections::HashMap;
use std::{path::Path, rc::Rc};

use __core::hash::Hash;
use __core::num::NonZeroU64;
use gltf::{Node, Result as GltfResult};
use rendiation_algebra::*;
use rendiation_scene_webgpu::{
  AnyMap, IntoStateControl, MeshDrawGroup, MeshModel, MeshModelImpl, PhysicalMaterial, Scene,
  SceneModelShareable, SceneNode, SceneTexture2D, StateControl, TextureWithSamplingData,
  WebGPUMesh, WebGPUScene,
};
use rendiation_scene_webgpu::{SceneModelHandle, WebGPUSceneExtension};
use shadergraph::*;
use webgpu::{
  create_gpu_buffer, GPUBufferResource, GPUBufferResourceView, GPUBufferViewRange,
  ShaderHashProvider, ShaderPassBuilder, TextureFormat, WebGPUTexture2dSource,
};

mod convert_utils;
use convert_utils::*;

pub struct GeometryBuffer {
  guid: u64,
  buffer: Vec<u8>,
}

/// like slice, but owned, ref counted cheap clone
#[derive(Clone)]
pub struct TypedBufferView {
  pub buffer: Rc<GeometryBuffer>,
  pub range: GPUBufferViewRange,
}

#[derive(Clone)]
pub struct AttributeAccessor {
  pub view: TypedBufferView,
  pub ty: gltf::accessor::DataType,
  pub dimension: gltf::accessor::Dimensions,
  pub start: usize,
  pub count: usize,
  pub stride: usize,
}

impl AttributeAccessor {
  fn compute_gpu_buffer_range(&self) -> GPUBufferViewRange {
    let inner_offset = self.view.range.offset;
    GPUBufferViewRange {
      offset: inner_offset + (self.start * self.stride) as u64,
      size: NonZeroU64::new(inner_offset + (self.count * self.stride) as u64)
        .unwrap() // safe
        .into(),
    }
  }
}

struct AttributesMesh {
  attributes: Vec<(gltf::Semantic, AttributeAccessor)>,
  indices: Option<AttributeAccessor>,
  mode: webgpu::PrimitiveTopology,
  // bounding: Box3<f32>,
}

struct AttributesMeshGPU {
  attributes: Vec<(gltf::Semantic, GPUBufferResourceView)>,
  indices: Option<(GPUBufferResourceView, webgpu::IndexFormat)>,
  mode: webgpu::PrimitiveTopology,
}

impl ShaderPassBuilder for AttributesMeshGPU {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    for (s, b) in &self.attributes {
      match s {
        gltf::Semantic::Positions => ctx.set_vertex_buffer_owned_next(b),
        gltf::Semantic::Normals => ctx.set_vertex_buffer_owned_next(b),
        gltf::Semantic::Tangents => {}
        gltf::Semantic::Colors(_) => ctx.set_vertex_buffer_owned_next(b),
        gltf::Semantic::TexCoords(_) => ctx.set_vertex_buffer_owned_next(b),
        gltf::Semantic::Joints(_) => {}
        gltf::Semantic::Weights(_) => {}
      }
    }
    if let Some((buffer, index_format)) = &self.indices {
      ctx.pass.set_index_buffer_owned(buffer, *index_format)
    }
  }
}

impl ShaderHashProvider for AttributesMeshGPU {
  fn hash_pipeline(&self, hasher: &mut webgpu::PipelineHasher) {
    for (s, _) in &self.attributes {
      s.hash(hasher)
    }
    self.mode.hash(hasher);
    if let Some((_, f)) = &self.indices {
      if webgpu::PrimitiveTopology::LineStrip == self.mode
        || webgpu::PrimitiveTopology::TriangleStrip == self.mode
      {
        f.hash(hasher)
      }
    }
  }
}
impl ShaderGraphProvider for AttributesMeshGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let mode = VertexStepMode::Vertex;
    builder.vertex(|builder, _| {
      for (s, _) in &self.attributes {
        match s {
          gltf::Semantic::Positions => builder.push_single_vertex_layout::<GeometryPosition>(mode),
          gltf::Semantic::Normals => builder.push_single_vertex_layout::<GeometryNormal>(mode),
          gltf::Semantic::Tangents => {}
          gltf::Semantic::Colors(_) => builder.push_single_vertex_layout::<GeometryColor>(mode),
          gltf::Semantic::TexCoords(_) => builder.push_single_vertex_layout::<GeometryUV>(mode),
          gltf::Semantic::Joints(_) => {}
          gltf::Semantic::Weights(_) => {}
        }
      }
      builder.primitive_state.topology = self.mode;
      Ok(())
    })
  }
}

// todo impl drop, cleanup
#[derive(Default)]
struct AttributesGPUCache {
  gpus: HashMap<u64, GPUBufferResource>,
}

impl AttributesGPUCache {
  pub fn get(&mut self, buffer: &GeometryBuffer, gpu: &webgpu::GPU) -> GPUBufferResource {
    self
      .gpus
      .entry(buffer.guid)
      .or_insert_with(|| {
        create_gpu_buffer(
          buffer.buffer.as_slice(),
          webgpu::BufferUsages::INDEX | webgpu::BufferUsages::VERTEX,
          &gpu.device,
        )
      })
      .clone()
  }
}

impl WebGPUMesh for AttributesMesh {
  type GPU = AttributesMeshGPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut AnyMap) -> Self::GPU {
    let cache: &mut AttributesGPUCache = storage.entry().or_insert_with(Default::default);

    let attributes = self
      .attributes
      .iter()
      .map(|(s, vertices)| {
        let buffer = cache.get(&vertices.view.buffer, gpu);
        let buffer_view = buffer.create_view(vertices.compute_gpu_buffer_range());
        (s.clone(), buffer_view)
      })
      .collect();

    let indices = self.indices.as_ref().map(|i| {
      let format = match i.ty {
        gltf::accessor::DataType::U16 => webgpu::IndexFormat::Uint16,
        gltf::accessor::DataType::U32 => webgpu::IndexFormat::Uint32,
        _ => unreachable!(),
      };
      let buffer = cache.get(&i.view.buffer, gpu);
      let buffer_view = buffer.create_view(i.compute_gpu_buffer_range());
      (buffer_view, format)
    });

    AttributesMeshGPU {
      attributes,
      indices,
      mode: self.mode,
    }
  }

  /// the current represent do not have meaningful mesh draw group concept
  fn draw_impl(&self, _group: MeshDrawGroup) -> webgpu::DrawCommand {
    if let Some(indices) = &self.indices {
      webgpu::DrawCommand::Indexed {
        base_vertex: 0,
        indices: 0..indices.count as u32,
        instances: 0..1,
      }
    } else {
      let attribute = &self.attributes.last().unwrap().1;
      webgpu::DrawCommand::Array {
        vertices: 0..attribute.count as u32,
        instances: 0..1,
      }
    }
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.mode
  }
}

fn build_model(
  node: &SceneNode,
  primitive: gltf::Primitive,
  ctx: &mut Context,
) -> impl SceneModelShareable {
  let attributes = primitive
    .attributes()
    .map(|(semantic, accessor)| (semantic, build_geom_buffer(accessor, ctx)))
    .collect();

  let indices = primitive
    .indices()
    .map(|indices| build_geom_buffer(indices, ctx));

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
    ty,
    stride,
    dimension,
  }
}

// todo use global lock
thread_local! {
  static UUID: core::cell::Cell<u64> = core::cell::Cell::new(0);
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
      .map(|buffer| {
        UUID.set(UUID.get() + 1);
        let buffer = GeometryBuffer {
          guid: UUID.get(),
          buffer: buffer.0,
        };
        Rc::new(buffer)
      })
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

impl WebGPUTexture2dSource for GltfImage {
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
