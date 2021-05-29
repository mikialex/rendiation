use rendiation_algebra::{Mat4, Projection};

use crate::renderer::Renderer;

pub struct Camera {
  pub projection: Box<dyn Projection>,
  pub projection_matrix: Mat4<f32>,
  pub matrix: Mat4<f32>, // to support scene graph we can associate a scene node with it
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
  pub fn new(renderer: &Renderer, camera: &Camera) -> Self {
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
