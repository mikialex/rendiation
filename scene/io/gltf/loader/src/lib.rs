#![feature(local_key_cell_methods)]

use std::collections::HashMap;
use std::{path::Path, rc::Rc};

use __core::hash::Hash;
use gltf::{Node, Result as GltfResult};
use rendiation_algebra::*;
use rendiation_scene_webgpu::{
  AnyMap, IntoStateControl, MeshDrawGroup, MeshModel, MeshModelImpl, PhysicalMaterial, Scene,
  SceneModelShareable, SceneNode, SceneTexture2D, StateControl, TextureWithSamplingData,
  WebGPUMesh, WebGPUScene,
};
use rendiation_scene_webgpu::{SceneModelHandle, WebGPUSceneExtension};
use shadergraph::*;
use webgpu::util::DeviceExt;
use webgpu::{ShaderHashProvider, ShaderPassBuilder, TextureFormat, WebGPUTexture2dSource};

struct GeometryBuffer {
  guid: u64,
  buffer: Vec<u8>,
}

/// like slice, but owned, ref counted cheap clone
#[derive(Clone)]
struct TypedBufferView {
  pub buffer: Rc<GeometryBuffer>,
  pub byte_start: usize,
  pub byte_count: usize,
  pub ty: gltf::accessor::DataType,
  pub dimension: gltf::accessor::Dimensions,
}

struct AttributesMesh {
  attributes: Vec<(gltf::Semantic, TypedBufferView)>,
  indices: Option<TypedBufferView>,
  mode: webgpu::PrimitiveTopology,
  // bounding: Box3<f32>,
}

struct AttributesMeshGPU {
  attributes: Vec<(gltf::Semantic, Rc<webgpu::Buffer>)>,
  indices: Option<(Rc<webgpu::Buffer>, webgpu::IndexFormat)>,
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
  gpus: HashMap<u64, Rc<webgpu::Buffer>>,
}

impl AttributesGPUCache {
  pub fn get(&mut self, buffer: &GeometryBuffer, gpu: &webgpu::GPU) -> Rc<webgpu::Buffer> {
    self
      .gpus
      .entry(buffer.guid)
      .or_insert_with(|| {
        Rc::new(
          gpu
            .device
            .create_buffer_init(&webgpu::util::BufferInitDescriptor {
              label: None,
              contents: buffer.buffer.as_slice(),
              usage: webgpu::BufferUsages::INDEX | webgpu::BufferUsages::VERTEX,
            }),
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
        let v = cache.get(&vertices.buffer, gpu);
        (s.clone(), v)
      })
      .collect();

    let indices = self.indices.as_ref().map(|i| {
      let format = match i.ty {
        gltf::accessor::DataType::U16 => webgpu::IndexFormat::Uint16,
        gltf::accessor::DataType::U32 => webgpu::IndexFormat::Uint32,
        _ => unreachable!(),
      };
      let v = cache.get(&i.buffer, gpu);
      (v, format)
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
        indices: indices.byte_start as u32..indices.byte_count as u32,
        instances: 0..1,
      }
    } else {
      let attribute = &self.attributes.last().unwrap().1;
      webgpu::DrawCommand::Array {
        vertices: attribute.byte_start as u32..attribute.byte_count as u32,
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

fn build_geom_buffer(accessor: gltf::Accessor, ctx: &mut Context) -> TypedBufferView {
  let index = accessor.index();
  let buffer = ctx.attributes[index].clone();

  UUID.set(buffer.guid + 1);

  let ty = accessor.data_type();
  let dimension = accessor.dimensions();

  let start = accessor.offset();
  let count = accessor.count();

  let byte_count = count
    * match ty {
      gltf::accessor::DataType::I8 => 1,
      gltf::accessor::DataType::U8 => 1,
      gltf::accessor::DataType::I16 => 2,
      gltf::accessor::DataType::U16 => 2,
      gltf::accessor::DataType::U32 => 4,
      gltf::accessor::DataType::F32 => 4,
    }
    * match dimension {
      gltf::accessor::Dimensions::Scalar => 1,
      gltf::accessor::Dimensions::Vec2 => 2,
      gltf::accessor::Dimensions::Vec3 => 3,
      gltf::accessor::Dimensions::Vec4 => 4,
      gltf::accessor::Dimensions::Mat2 => 4,
      gltf::accessor::Dimensions::Mat3 => 9,
      gltf::accessor::Dimensions::Mat4 => 16,
    };

  TypedBufferView {
    buffer,
    byte_start: start,
    byte_count,
    ty,
    dimension,
  }
}

fn map_draw_mode(mode: gltf::mesh::Mode) -> Option<webgpu::PrimitiveTopology> {
  match mode {
    gltf::mesh::Mode::Points => webgpu::PrimitiveTopology::PointList,
    gltf::mesh::Mode::Lines => webgpu::PrimitiveTopology::LineList,
    gltf::mesh::Mode::LineLoop => return None,
    gltf::mesh::Mode::LineStrip => webgpu::PrimitiveTopology::LineStrip,
    gltf::mesh::Mode::Triangles => webgpu::PrimitiveTopology::TriangleList,
    gltf::mesh::Mode::TriangleStrip => webgpu::PrimitiveTopology::TriangleStrip,
    gltf::mesh::Mode::TriangleFan => return None,
  }
  .into()
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

  let image = GltfImage {
    data: data.pixels,
    format,
    size,
  };
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
