use std::rc::Rc;

use bytemuck::{Pod, Zeroable};
use rendiation_algebra::*;
use rendiation_webgpu::*;

use crate::*;

impl CameraViewBounds {
  pub fn setup_viewport<'a>(&self, pass: &mut GPURenderPass<'a>) {
    let size = pass.info().buffer_size;
    let width: usize = size.width.into();
    let width = width as f32;
    let height: usize = size.height.into();
    let height = height as f32;
    pass.set_viewport(
      width * self.to_left,
      height * self.to_top,
      width * self.width,
      height * self.height,
      0.,
      1.,
    )
  }
}

#[derive(Default)]
pub struct CameraGPU {
  inner: ResourceMapper<CameraBindgroup, Camera>,
}

impl std::ops::Deref for CameraGPU {
  type Target = ResourceMapper<CameraBindgroup, Camera>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for CameraGPU {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl CameraGPU {
  pub fn check_update_gpu(&mut self, camera: &mut SceneCamera, gpu: &GPU) -> &CameraBindgroup {
    self.get_update_or_insert_with(
      camera,
      |_| CameraBindgroup::new(gpu),
      |camera_gpu, camera| {
        camera_gpu.update(gpu, camera);
      },
    )
  }

  pub fn expect_gpu(&self, camera: &SceneCamera) -> &CameraBindgroup {
    self.get_unwrap(camera)
  }
}

pub struct CameraBindgroup {
  pub ubo: UniformBufferData<CameraGPUTransform>,
  pub bindgroup: Rc<wgpu::BindGroup>,
}

pub struct ClipPosition;
impl SemanticVertexShaderValue for ClipPosition {
  type ValueType = Vec4<f32>;
}

impl ShaderGraphProvider for CameraBindgroup {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let camera = builder.register_uniform::<CameraGPUTransform>().expand();
    let position = builder.query::<WorldVertexPosition>()?;
    builder.register::<ClipPosition>(camera.projection * camera.view * (position, 1.).into());
    Ok(())
  }
}

impl BindGroupLayoutProvider for CameraBindgroup {
  fn bind_preference() -> usize {
    2
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "CameraBindgroup".into(),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: wgpu::BufferSize::new(64 * 3),
        },
        count: None,
      }],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "

      [[group({group}), binding(0)]]
      var<uniform> camera: CameraTransform;
    
    "
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<CameraGPUTransform>();
  }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, ShaderUniform)]
pub struct CameraGPUTransform {
  projection: Mat4<f32>,
  rotation: Mat4<f32>,
  view: Mat4<f32>,
}

impl ShaderUniformBlock for CameraGPUTransform {
  fn shader_struct() -> &'static str {
    "
      struct CameraTransform {
        projection: mat4x4<f32>;
        rotation:   mat4x4<f32>;
        view:       mat4x4<f32>;
      };
      "
  }
}

impl CameraBindgroup {
  pub fn update(&mut self, gpu: &GPU, camera: &Camera) -> &mut Self {
    let uniform: &mut CameraGPUTransform = &mut self.ubo;
    let world_matrix = camera.node.visit(|node| node.local_matrix);
    uniform.view = world_matrix.inverse_or_identity();
    uniform.rotation = world_matrix.extract_rotation_mat();
    uniform.projection = camera.projection_matrix;

    self.ubo.update(&gpu.queue);

    self
  }

  pub fn new(gpu: &GPU) -> Self {
    let device = &gpu.device;

    let ubo: UniformBufferData<CameraGPUTransform> = UniformBufferData::create_default(device);

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &Self::layout(device),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: ubo.as_bindable(),
      }],
      label: None,
    });
    let bindgroup = Rc::new(bindgroup);

    Self { ubo, bindgroup }
  }
}
