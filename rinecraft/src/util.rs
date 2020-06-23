use image::ImageBuffer;
use image::Rgba;
use rendiation::consts::OPENGL_TO_WGPU_MATRIX;
use rendiation::*;
use rendiation_math::{Vec2, Vec3};
use rendiation_render_entity::*;
use rendiation_scenegraph::{Scene, UniformHandle, WebGPUBackend};

pub struct CameraGPU {
  pub gpu_camera_position: UniformHandle<WebGPUBackend>,
  gpu_camera_position_dirty: bool,
  pub gpu_mvp_matrix: UniformHandle<WebGPUBackend>,
  gpu_mvp_matrix_dirty: bool,
}

impl CameraGPU {
  pub fn new(
    renderer: &WGPURenderer,
    camera: &PerspectiveCamera,
    scene: &mut Scene<WebGPUBackend>,
  ) -> Self {
    let gpu_camera_position = WGPUBuffer::new(
      renderer,
      CameraGPU::get_world_position_data(camera),
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();

    let gpu_mvp_matrix = WGPUBuffer::new(
      renderer,
      mx_total.as_ref(),
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );
    Self {
      gpu_camera_position: scene.resources.add_uniform(gpu_camera_position).index(),
      gpu_camera_position_dirty: false,
      gpu_mvp_matrix: scene.resources.add_uniform(gpu_mvp_matrix).index(),
      gpu_mvp_matrix_dirty: false,
    }
  }

  pub fn mark_dirty(&mut self) {
    self.gpu_mvp_matrix_dirty = true;
    self.gpu_camera_position_dirty = true;
  }

  fn get_world_position_data(camera: &impl Camera) -> &[u8] {
    let transform = camera.get_transform();
    transform.position.as_ref()
  }

  pub fn update_gpu_world_position(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WebGPUBackend>,
  ) {
    let camera = scene.cameras.get_active_camera_mut::<PerspectiveCamera>();
    let data = CameraGPU::get_world_position_data(camera);
    self.gpu_camera_position_dirty = false;
    scene
      .resources
      .get_uniform_mut(self.gpu_camera_position)
      .resource_mut()
      .update(renderer, data);
  }

  pub fn update_gpu_mvp_matrix(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WebGPUBackend>,
  ) {
    let camera = scene.cameras.get_active_camera_mut::<PerspectiveCamera>();
    self.gpu_mvp_matrix_dirty = false;

    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();

    scene
      .resources
      .get_uniform_mut(self.gpu_mvp_matrix)
      .resource_mut()
      .update(renderer, mx_total.as_ref());
  }

  pub fn update_all(&mut self, renderer: &mut WGPURenderer, scene: &mut Scene<WebGPUBackend>) {
    self.update_gpu_mvp_matrix(renderer, scene);
    self.update_gpu_world_position(renderer, scene);
  }
}

pub fn create_texels(size: usize) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
  use std::iter;

  let data = (0..size * size)
    .flat_map(|id| {
      // get high five for recognizing this ;)
      let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
      let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
      let (mut x, mut y, mut count) = (cx, cy, 0);
      while count < 0xFF && x * x + y * y < 4.0 {
        let old_x = x;
        x = x * x - y * y + cx;
        y = 2.0 * old_x * y + cy;
        count += 1;
      }
      iter::once(0xFF - (count * 5) as u8)
        .chain(iter::once(0xFF - (count * 15) as u8))
        .chain(iter::once(0xFF - (count * 50) as u8))
        .chain(iter::once(1))
    })
    .collect();
  image::ImageBuffer::from_raw(size as u32, size as u32, data).unwrap()
}

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
  use std::mem::size_of;
  use std::slice::from_raw_parts;

  unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}
