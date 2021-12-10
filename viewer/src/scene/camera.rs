use std::{ops::Deref, rc::Rc};

use bytemuck::{Pod, Zeroable};
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::{
  helpers::{ColoredLineVertex, HelperLineMesh},
  SceneNode,
};

pub trait CameraProjection {
  fn update_projection(&self, projection: &mut Mat4<f32>);
  fn resize(&mut self, size: (f32, f32));
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32>;
}

impl<T: ResizableProjection + RayCaster3<f32>> CameraProjection for T {
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    self.update_projection::<WebGPU>(projection);
  }
  fn resize(&mut self, size: (f32, f32)) {
    self.resize(size);
  }
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32 {
    self.pixels_per_unit(distance, view_height)
  }

  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.cast_ray(normalized_position)
  }
}

pub struct CameraViewBounds {
  pub width: f32,
  pub height: f32,
  pub to_left: f32,
  pub to_top: f32,
}

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

impl Default for CameraViewBounds {
  fn default() -> Self {
    Self {
      width: 1.,
      height: 1.,
      to_left: 0.,
      to_top: 0.,
    }
  }
}

pub struct Camera {
  pub bounds: CameraViewBounds,
  pub projection: Box<dyn CameraProjection>,
  pub projection_matrix: Mat4<f32>,
  pub node: SceneNode,
}

impl Camera {
  pub fn view_size_in_pixel(&self, frame_size: Size) -> Vec2<f32> {
    let width: usize = frame_size.width.into();
    let width = width as f32 * self.bounds.width;
    let height: usize = frame_size.height.into();
    let height = height as f32 * self.bounds.height;
    (width, height).into()
  }

  pub fn build_debug_line_in_camera_space(&self) -> HelperLineMesh {
    let near_left_down = Vec3::new(0., 0., 0.);
    let near_left_top = Vec3::new(0., 0., 0.);
    let near_right_down = Vec3::new(0., 0., 0.);
    let near_right_top = Vec3::new(0., 0., 0.);

    let far_left_down = Vec3::new(0., 0., 0.);
    let far_left_top = Vec3::new(0., 0., 0.);
    let far_right_down = Vec3::new(0., 0., 0.);
    let far_right_top = Vec3::new(0., 0., 0.);

    let lines = vec![
      [near_left_down, near_left_top],
      [near_right_down, near_right_top],
      [near_left_down, near_right_down],
      [near_left_top, near_right_top],
      //
      [far_left_down, far_left_top],
      [far_right_down, far_right_top],
      [far_left_down, far_right_down],
      [far_left_top, far_right_top],
      //
      [near_left_down, far_left_down],
      [near_left_top, far_left_top],
      [near_right_down, far_right_down],
      [near_right_top, far_right_top],
    ];

    let line_buffer = lines
      .iter()
      .flat_map(|line| line.iter())
      .map(|&v| ColoredLineVertex {
        position: v,
        color: Vec4::new(1., 1., 1., 1.),
      })
      .collect();

    HelperLineMesh::new(line_buffer)
  }
}

pub struct SceneCamera {
  cpu: Camera,
  gpu: Option<CameraBindgroup>,
}

impl Deref for SceneCamera {
  type Target = Camera;

  fn deref(&self) -> &Self::Target {
    &self.cpu
  }
}

impl std::ops::DerefMut for SceneCamera {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.cpu
  }
}

impl SceneCamera {
  pub fn new(p: impl ResizableProjection + RayCaster3<f32> + 'static, node: SceneNode) -> Self {
    Self {
      cpu: Camera {
        bounds: Default::default(),
        projection: Box::new(p),
        projection_matrix: Mat4::one(),
        node,
      },
      gpu: None,
    }
  }

  pub fn resize(&mut self, size: (f32, f32)) {
    self.projection.resize(size);
  }

  pub fn cast_world_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.projection.cast_ray(normalized_position)
  }

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
