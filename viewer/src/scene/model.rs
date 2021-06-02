use arena::Arena;
use rendiation_algebra::*;

use super::*;

pub struct Model {
  pub(crate) material: MaterialHandle,
  pub(crate) mesh: MeshHandle,
  pub node: SceneNodeHandle,
}

pub struct ModelPassSetupContext<'a, S> {
  pub materials: &'a Arena<Box<dyn Material>>,
  pub meshes: &'a Arena<SceneMesh>,
  pub material_ctx: SceneMaterialPassSetupCtx<'a, S>,
}

pub struct ModelTransformGPU {
  pub ubo: wgpu::Buffer,
  pub bindgroup: wgpu::BindGroup,
  pub layout: wgpu::BindGroupLayout,
}

impl ModelTransformGPU {
  pub fn get_shader_header() -> &'static str {
    r#"
      [[block]]
      struct ModelTransform {
          matrix: mat4x4<f32>;
      };
      [[group(0), binding(0)]]
      var model: ModelTransform;
    "#
  }

  pub fn update(&mut self, renderer: &Renderer, matrix: &Mat4<f32>) {
    renderer
      .queue
      .write_buffer(&self.ubo, 0, bytemuck::cast_slice(matrix.as_ref()));
  }
  pub fn new(renderer: &Renderer, matrix: &Mat4<f32>) -> Self {
    let device = &renderer.device;
    use wgpu::util::DeviceExt;

    let ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: "ModelTransformBindgroup Buffer".into(),
      contents: bytemuck::cast_slice(matrix.as_ref()),
      usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "ModelTransformBindgroup".into(),
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
