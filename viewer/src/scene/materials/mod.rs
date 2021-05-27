pub mod basic;
pub use basic::*;
use rendiation_algebra::Mat4;

use crate::Renderer;

use super::Camera;

// pub trait MaterialRenderable<PassSchema, Vertex>{

// }

pub trait MaterialCPUResource {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn setup_bindgroup<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
  fn setup_pipeline<'a>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
  );
}

pub struct MaterialCell<T: MaterialCPUResource> {
  material: T,
  gpu: T::GPU,
}

pub struct SceneMaterialRenderPrepareCtx<'a> {
  pub camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub model_matrix_gpu: &'a wgpu::Buffer,
  pub pipelines: &'a mut PipelineResourceManager,
}

pub struct CameraBindgroup {
  pub uniform_buf: wgpu::Buffer,
  pub bindgroup: wgpu::BindGroup,
  pub layout: wgpu::BindGroupLayout,
}

impl CameraBindgroup {
  pub fn bindgroup_shader_header() -> &'static str {
    r#"
      [[block]]
      struct CameraTransform {
          projection: mat4x4<f32>;
      };
      [[group(0), binding(0)]]
      var camera: CameraTransform;
    "#
  }
  pub fn update(&mut self, renderer: &Renderer, camera: &Camera) {
    renderer.queue.write_buffer(
      &self.uniform_buf,
      0,
      bytemuck::cast_slice(camera.projection_matrix.as_ref()),
    );
  }
  pub fn new(&mut self, renderer: &Renderer, camera: &Camera) -> Self {
    let device = &renderer.device;
    use wgpu::util::DeviceExt;

    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: "CameraBindgroup Buffer".into(),
      contents: bytemuck::cast_slice(camera.projection_matrix.as_ref()),
      usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "CameraBindgroup".into(),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStage::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: wgpu::BufferSize::new(64),
        },
        count: None,
      }],
    });

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform_buf.as_entire_binding(),
      }],
      label: None,
    });

    Self {
      uniform_buf,
      bindgroup,
      layout,
    }
  }
}

pub trait Material {
  fn update<'a>(&mut self, renderer: &Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a>);
  fn setup_bindgroup<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}

impl<T> Material for MaterialCell<T>
where
  T: MaterialCPUResource,
{
  fn update<'a>(&mut self, renderer: &Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a>) {
    self.gpu.update(&self.material, renderer, ctx);
  }
  fn setup_bindgroup<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    self.gpu.setup_bindgroup(pass)
  }
}

pub struct PipelineResourceManager {
  basic: Option<wgpu::RenderPipeline>,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self { basic: None }
  }
}
