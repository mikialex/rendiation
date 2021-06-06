use arena_tree::ArenaTree;
use rendiation_algebra::*;

use super::{SceneNode, SceneNodeHandle};
use crate::renderer::Renderer;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4<f32> = Mat4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
  pub projection: Box<dyn Projection>,
  pub projection_matrix: Mat4<f32>,
  pub node: SceneNodeHandle,
}

impl Camera {
  pub fn new(p: impl Projection + 'static, node: SceneNodeHandle) -> Self {
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
    self.projection_matrix = OPENGL_TO_WGPU_MATRIX * self.projection_matrix;
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
  pub fn update(
    &mut self,
    renderer: &Renderer,
    camera: &Camera,
    nodes: &ArenaTree<SceneNode>,
  ) -> &mut Self {
    renderer.queue.write_buffer(
      &self.ubo,
      0,
      bytemuck::cast_slice(camera.projection_matrix.as_ref()),
    );
    renderer.queue.write_buffer(
      &self.ubo,
      64,
      bytemuck::cast_slice(camera.get_view_matrix(nodes).as_ref()),
    );
    self
  }
  pub fn new(renderer: &Renderer, camera: &Camera) -> Self {
    let device = &renderer.device;
    use wgpu::util::DeviceExt;

    let mat = [0_u8; 128];

    let ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: "CameraBindgroup Buffer".into(),
      contents: &mat,
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
