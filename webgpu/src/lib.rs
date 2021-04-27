use std::any::Any;

use rendiation_algebra::Vec3;
use sceno::{Arena, Handle, NextTraverseVisit, SceneBackend};
use swap_chain::SwapChain;
use wgpu::util::DeviceExt;

pub struct WebGPUScene;

mod swap_chain;

impl SceneBackend for WebGPUScene {
  type Model = Box<dyn Model>;
  type Material = Box<dyn Material>;
  type Mesh = Box<dyn Mesh>;
  type Background = Box<dyn Background>;
  type Light = Box<dyn Light>;
}

pub type Scene = sceno::Scene<WebGPUScene>;
pub type SceneNode = sceno::SceneNode<WebGPUScene>;
pub type NodeHandle = sceno::SceneNodeHandle<WebGPUScene>;
pub type MeshHandle = sceno::MeshHandle<WebGPUScene>;
pub type MaterialHandle = sceno::MaterialHandle<WebGPUScene>;

pub trait Light {}
pub trait Background {}
pub trait Mesh {
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>);
}
pub trait Material {
  fn setup_pass<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
    ctx: &mut SceneMaterialRenderCtx,
  ) {
    let pipeline = self.get_pipeline(des, renderer);
    pass.set_pipeline(pipeline);
    self.setup_bindgroups(renderer, pass, ctx);
  }

  fn get_pipeline<'a>(
    &mut self,
    des: &wgpu::RenderPassDescriptor,
    renderer: &'a Renderer,
  ) -> &'a wgpu::RenderPipeline;

  fn setup_bindgroups<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &mut SceneMaterialRenderCtx,
  );
}
pub trait Model {
  fn render<'a>(
    &self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
    ctx: &mut SceneRenderCtx,
  );
}

pub trait GPUSceneExt {
  //
}

pub struct Renderer {
  instance: wgpu::Instance,
  adaptor: wgpu::Adapter,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: SwapChain,

  pipeline_cache: Vec<wgpu::RenderPipeline>,
  // bindgroup_cache: Vec<wgpu::BindGroup>,
  buffers: Arena<wgpu::Buffer>,
}

impl Renderer {
  pub async fn new(window: &winit::window::Window) -> Self {
    let backend = wgpu::BackendBit::PRIMARY;
    let instance = wgpu::Instance::new(backend);
    let power_preference = wgpu::PowerPreference::default();

    let (size, surface) = unsafe {
      let size = window.inner_size();
      let surface = instance.create_surface(window);
      (size, surface)
    };
    let adaptor = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: Some(&surface),
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adaptor
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await
      .expect("Unable to find a suitable GPU device!");

    let swap_chain = SwapChain::new(
      &adaptor,
      &device,
      surface,
      (size.width as usize, size.height as usize),
    );

    Self {
      pipeline_cache: Vec::new(),
      buffers: Arena::new(),
      instance,
      adaptor,
      device,
      queue,
      swap_chain,
    }
  }
  pub fn render(&mut self, pass_des: &wgpu::RenderPassDescriptor, renderable: &mut dyn Renderable) {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
      let mut pass = encoder.begin_render_pass(pass_des);
      renderable.render(self, &mut pass, pass_des);
    }

    self.queue.submit(Some(encoder.finish()));
  }
  pub fn resize(&mut self, size: (usize, usize)) {
    self.swap_chain.resize(size, &self.device);
  }
}

pub trait Renderable {
  fn render<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
  );
}

pub struct SceneRenderCtx<'a> {
  materials: &'a mut Arena<Box<dyn Material>>,
  meshes: &'a mut Arena<Box<dyn Mesh>>,
  material_ctx: SceneMaterialRenderCtx,
}

pub struct SceneMaterialRenderCtx {}

impl Renderable for Scene {
  fn render<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
  ) {
    self.update();

    let root = self.get_root_handle();
    let nodes = &mut self.nodes;
    let models = &self.models;
    let mut ctx = SceneRenderCtx {
      materials: &mut self.materials,
      meshes: &mut self.meshes,
      material_ctx: SceneMaterialRenderCtx {},
    };
    let mut model_list = Vec::new();
    nodes.traverse(root, &mut Vec::new(), |node, _| {
      let node = node.data();
      node.payload.iter().for_each(|payload| match payload {
        sceno::SceneNodePayload::Model(model) => {
          model_list.push(*model);
        }
        _ => {}
      });
      NextTraverseVisit::VisitChildren
    });
    model_list.iter().for_each(|model| {
      let model = models.get(*model).unwrap();
      model.render(renderer, pass, des, &mut ctx)
    })
  }
}

// trait MeshBufferBackend: SceneBackend {
//   type VertexBuffer;
//   type VertexBufferGPU;
// }

// impl MeshBufferBackend for WebGPUScene {
//   type VertexBuffer = Box<dyn VertexBufferSource>;
//   type VertexBufferGPU = wgpu::Buffer;
// }

pub trait VertexBufferSource: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_bytes(&self) -> &[u8];
  fn get_layout(&self) -> wgpu::VertexBufferLayout;
}
pub struct VertexBuffer {
  data: Box<dyn VertexBufferSource>,
  gpu: Option<Handle<wgpu::Buffer>>,
}

impl VertexBuffer {
  pub fn new(data: impl VertexBufferSource) -> Self {
    let data = Box::new(data);
    Self { data, gpu: None }
  }

  pub fn update(&mut self, renderer: &mut Renderer) {
    let data = &self.data;
    self.gpu.get_or_insert_with(|| {
      let device = &renderer.device;
      let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data.as_bytes(),
        usage: wgpu::BufferUsage::VERTEX,
      });
      renderer.buffers.insert(gpu)
    });
  }

  pub fn setup_pass<'a>(&self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>, slot: u32) {
    let gpu = self.gpu.unwrap();
    let gpu = renderer.buffers.get(gpu).unwrap();
    pass.set_vertex_buffer(slot, gpu.slice(..));
  }
}

pub trait IndexBufferSource: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_bytes(&self) -> &[u8];
  fn index_format(&self) -> wgpu::IndexFormat;
}

pub struct IndexBuffer {
  data: Box<dyn IndexBufferSource>,
  gpu: Option<Handle<wgpu::Buffer>>,
}

impl IndexBuffer {
  pub fn new(data: impl IndexBufferSource) -> Self {
    let data = Box::new(data);
    Self { data, gpu: None }
  }

  pub fn update(&mut self, renderer: &mut Renderer) {
    let data = &self.data;
    self.gpu.get_or_insert_with(|| {
      let device = &renderer.device;
      let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data.as_bytes(),
        usage: wgpu::BufferUsage::INDEX,
      });
      renderer.buffers.insert(gpu)
    });
  }

  pub fn setup_pass<'a>(&self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>) {
    let gpu = self.gpu.unwrap();
    let gpu = renderer.buffers.get(gpu).unwrap();
    pass.set_index_buffer(gpu.slice(..), self.data.index_format());
  }
}

pub struct SceneMesh {
  vertex: Vec<VertexBuffer>,
  index: Option<IndexBuffer>,
}

impl Mesh for SceneMesh {
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>) {
    self
      .index
      .as_mut()
      .map(|index| index.setup_pass(renderer, pass));
    self
      .vertex
      .iter_mut()
      .enumerate()
      .for_each(|(i, vertex)| vertex.setup_pass(renderer, pass, i as u32))
  }
}

pub struct SceneModel {
  material: MaterialHandle,
  mesh: MeshHandle,
}

impl Model for SceneModel {
  fn render<'a>(
    &self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
    ctx: &mut SceneRenderCtx,
  ) {
    let material = ctx.materials.get_mut(self.material).unwrap();
    material.setup_pass(renderer, pass, des, &mut ctx.material_ctx);
    let mesh = ctx.meshes.get_mut(self.mesh).unwrap();
    mesh.setup_pass(renderer, pass);
  }
}

struct BasicMaterial {
  pub color: Vec3<f32>,
}

struct BasicMaterialGPU {
  self_bindgroup: wgpu::BindGroup,
  // pipeline
}

pub trait GPUMaterial {
  type GPU;
}

struct GPUMaterialWrap<T: GPUMaterial> {
  material: T,
  gpu: T::GPU,
}

impl<T: GPUMaterial> Material for GPUMaterialWrap<T> {
  fn get_pipeline<'a>(
    &mut self,
    des: &wgpu::RenderPassDescriptor,
    renderer: &'a Renderer,
  ) -> &'a wgpu::RenderPipeline {
    let bind_group_layout =
      renderer
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          label: None,
          entries: &[
            wgpu::BindGroupLayoutEntry {
              binding: 0,
              visibility: wgpu::ShaderStage::VERTEX,
              ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(64),
              },
              count: None,
            },
            wgpu::BindGroupLayoutEntry {
              binding: 1,
              visibility: wgpu::ShaderStage::FRAGMENT,
              ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
              },
              count: None,
            },
            wgpu::BindGroupLayoutEntry {
              binding: 2,
              visibility: wgpu::ShaderStage::FRAGMENT,
              ty: wgpu::BindingType::Sampler {
                comparison: false,
                filtering: true,
              },
              count: None,
            },
          ],
        });

    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
      });

    // let pipeline_des = wgpu::RenderPipelineDescriptor {
    //   label: None,
    //   layout: Some(&pipeline_layout),
    //   vertex: wgpu::VertexState {
    //     module: &shader,
    //     entry_point: "vs_main",
    //     buffers: &vertex_buffers,
    //   },
    //   fragment: Some(wgpu::FragmentState {
    //     module: &shader,
    //     entry_point: "fs_main",
    //     targets: &[sc_desc.format.into()],
    //   }),
    //   primitive: wgpu::PrimitiveState {
    //     cull_mode: wgpu::CullMode::Back,
    //     ..Default::default()
    //   },
    //   depth_stencil: None,
    //   multisample: wgpu::MultisampleState::default(),
    // };
    //
    todo!()
  }

  fn setup_bindgroups<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &mut SceneMaterialRenderCtx,
  ) {
    todo!()
  }
  //
}
