use arena_tree::ArenaTree;
use rendiation_algebra::*;

use super::{SceneNode, SceneNodeHandle};
use rendiation_webgpu::*;

pub trait CameraProjection {
  fn update_projection(&self, projection: &mut Mat4<f32>);
  fn resize(&mut self, size: (f32, f32));
}

impl<T: ResizableProjection> CameraProjection for T {
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    self.update_projection::<WebGPU>(projection);
  }
  fn resize(&mut self, size: (f32, f32)) {
    self.resize(size);
  }
}

pub struct Camera {
  pub projection: Box<dyn CameraProjection>,
  pub projection_matrix: Mat4<f32>,
  pub node: SceneNodeHandle,
}

impl Camera {
  pub fn new(p: impl ResizableProjection + 'static, node: SceneNodeHandle) -> Self {
    Self {
      projection: Box::new(p),
      projection_matrix: Mat4::one(),
      node,
    }
  }

  pub fn update(&mut self) {
    self
      .projection
      .update_projection(&mut self.projection_matrix);
  }

  pub fn get_view_matrix(&self, nodes: &ArenaTree<SceneNode>) -> Mat4<f32> {
    nodes
      .get_node(self.node)
      .data()
      .world_matrix
      .inverse_or_identity()
  }
}

pub struct CameraBindgroup {
  pub ubo: wgpu::Buffer,
  pub bindgroup: wgpu::BindGroup,
  pub layout: wgpu::BindGroupLayout,
}

impl CameraBindgroup {
  pub fn get_shader_header() -> &'static str {
    r#"
      [[block]]
      struct CameraTransform {
          projection: mat4x4<f32>;
          view:       mat4x4<f32>;
      };
      [[group(2), binding(0)]]
      var camera: CameraTransform;
    "#
  }
  pub fn update(&mut self, gpu: &GPU, camera: &Camera, nodes: &ArenaTree<SceneNode>) -> &mut Self {
    gpu.queue.write_buffer(
      &self.ubo,
      0,
      bytemuck::cast_slice(camera.projection_matrix.as_ref()),
    );
    gpu.queue.write_buffer(
      &self.ubo,
      64,
      bytemuck::cast_slice(camera.get_view_matrix(nodes).as_ref()),
    );
    self
  }
  pub fn new(gpu: &GPU) -> Self {
    let device = &gpu.device;
    use wgpu::util::DeviceExt;

    let mat = [0_u8; 128];

    let ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: "CameraBindgroup Buffer".into(),
      contents: &mat,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "CameraBindgroup".into(),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: wgpu::BufferSize::new(64 * 2),
        },
        count: None,
      }],
    });

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: ubo.as_entire_binding(),
      }],
      label: None,
    });

    Self {
      ubo,
      bindgroup,
      layout,
    }
  }
}
