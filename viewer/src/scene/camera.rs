use std::ops::Deref;

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

pub struct CameraData {
  pub projection: Box<dyn CameraProjection>,
  pub projection_matrix: Mat4<f32>,
  pub node: SceneNodeHandle,
}

impl CameraData {
  pub fn get_view_matrix(&self, nodes: &ArenaTree<SceneNode>) -> Mat4<f32> {
    nodes
      .get_node(self.node)
      .data()
      .world_matrix
      .inverse_or_identity()
  }
}

pub struct Camera {
  cpu: CameraData,
  gpu: Option<CameraBindgroup>,
}

impl Deref for Camera {
  type Target = CameraData;

  fn deref(&self) -> &Self::Target {
    &self.cpu
  }
}

impl std::ops::DerefMut for Camera {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.cpu
  }
}

impl Camera {
  pub fn new(p: impl ResizableProjection + 'static, node: SceneNodeHandle) -> Self {
    Self {
      cpu: CameraData {
        projection: Box::new(p),
        projection_matrix: Mat4::one(),
        node,
      },
      gpu: None,
    }
  }

  pub fn get_updated_gpu(
    &mut self,
    gpu: &GPU,
    nodes: &ArenaTree<SceneNode>,
  ) -> (&CameraData, &mut CameraBindgroup) {
    self
      .gpu
      .get_or_insert_with(|| CameraBindgroup::new(gpu))
      .update(gpu, &mut self.cpu, nodes)
  }

  pub fn expect_gpu(&self) -> &CameraBindgroup {
    self.gpu.as_ref().unwrap()
  }
}

pub struct CameraBindgroup {
  pub ubo: wgpu::Buffer,
  pub bindgroup: wgpu::BindGroup,
}

impl BindGroupLayoutProvider for CameraBindgroup {
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
}

impl CameraBindgroup {
  pub fn get_shader_header() -> &'static str {
    r#"
      [[block]]
      struct CameraTransform {
          projection: mat4x4<f32>;
          rotation:   mat4x4<f32>;
          view:       mat4x4<f32>;
      };
      [[group(2), binding(0)]]
      var<uniform> camera: CameraTransform;
    "#
  }
  pub fn update<'a>(
    &mut self,
    gpu: &GPU,
    camera: &'a mut CameraData,
    nodes: &ArenaTree<SceneNode>,
  ) -> (&'a CameraData, &mut Self) {
    camera
      .projection
      .update_projection(&mut camera.projection_matrix);

    let world_matrix = nodes.get_node(camera.node).data().world_matrix;
    let view_matrix = world_matrix.inverse_or_identity();
    let rotation_matrix = world_matrix.extract_rotation_mat();

    gpu.queue.write_buffer(
      &self.ubo,
      0,
      bytemuck::cast_slice(camera.projection_matrix.as_ref()),
    );
    gpu.queue.write_buffer(
      &self.ubo,
      64,
      bytemuck::cast_slice(rotation_matrix.as_ref()),
    );
    gpu.queue.write_buffer(
      &self.ubo,
      64 + 64,
      bytemuck::cast_slice(view_matrix.as_ref()),
    );
    (camera, self)
  }

  pub fn new(gpu: &GPU) -> Self {
    let device = &gpu.device;
    use wgpu::util::DeviceExt;

    let mat = [0_u8; 64 * 3];

    let ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: "CameraBindgroup Buffer".into(),
      contents: &mat,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &Self::layout(device),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: ubo.as_entire_binding(),
      }],
      label: None,
    });

    Self { ubo, bindgroup }
  }
}
