use std::{ops::Deref, rc::Rc};

use bytemuck::{Pod, Zeroable};
use rendiation_algebra::*;
use rendiation_geometry::*;
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

// impl SceneRenderable for Camera {
//   fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
//     let helper = self.helper_object.get_or_insert_with(|| {
//       CameraHelper::from_node_and_project_matrix(self.node.clone(), self.projection_matrix)
//     });
//     helper.mesh.update(gpu, base)
//   }

//   fn setup_pass<'a>(
//     &self,
//     pass: &mut SceneRenderPass<'a>,
//     camera_gpu: &CameraBindgroup,
//     resources: &GPUResourceCache,
//   ) {
//     let helper = self.helper_object.as_ref().unwrap();
//     helper.mesh.setup_pass(pass, camera_gpu, resources)
//   }
// }

impl SceneCamera {
  pub fn get_updated_gpu(&mut self, gpu: &GPU) -> (&Camera, &mut CameraBindgroup) {
    self
      .gpu
      .get_or_insert_with(|| CameraBindgroup::new(gpu))
      .update(gpu, &mut self.cpu)
  }

  pub fn expect_gpu(&self) -> &CameraBindgroup {
    self.gpu.as_ref().unwrap()
  }
}

pub struct CameraBindgroup {
  pub ubo: UniformBufferData<CameraGPUTransform>,
  pub bindgroup: Rc<wgpu::BindGroup>,
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
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct CameraGPUTransform {
  projection: Mat4<f32>,
  rotation: Mat4<f32>,
  view: Mat4<f32>,
}

impl ShaderUniformBlock for CameraGPUTransform {
  fn shader_struct() -> &'static str {
    "
      [[block]]
      struct CameraTransform {
        projection: mat4x4<f32>;
        rotation:   mat4x4<f32>;
        view:       mat4x4<f32>;
      };
      "
  }
}

impl CameraBindgroup {
  pub fn update<'a>(&mut self, gpu: &GPU, camera: &'a mut Camera) -> (&'a Camera, &mut Self) {
    camera
      .projection
      .update_projection(&mut camera.projection_matrix);

    let uniform: &mut CameraGPUTransform = &mut self.ubo;
    let world_matrix = camera.node.visit(|node| node.local_matrix);
    uniform.view = world_matrix.inverse_or_identity();
    uniform.rotation = world_matrix.extract_rotation_mat();
    uniform.projection = camera.projection_matrix;

    self.ubo.update(&gpu.queue);

    (camera, self)
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
