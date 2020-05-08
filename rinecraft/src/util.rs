use crate::watch::GPUItem;
use image::ImageBuffer;
use image::Rgba;
use rendiation::consts::OPENGL_TO_WGPU_MATRIX;
use rendiation::*;
use rendiation_math::{Vec2, Vec3};
use rendiation_render_entity::*;

pub struct CameraGPU {
  pub gpu_camera_position: WGPUBuffer,
  gpu_camera_position_dirty: bool,
  pub gpu_mvp_matrix: WGPUBuffer,
  gpu_mvp_matrix_dirty: bool,
}

impl CameraGPU {
  pub fn new(renderer: &WGPURenderer, camera: &PerspectiveCamera) -> Self {
    let gpu_camera_position = WGPUBuffer::new(
      renderer,
      CameraGPU::get_world_position_data(camera),
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();
    let mx_total_ref: &[f32; 16] = mx_total.as_ref();

    let gpu_mvp_matrix = WGPUBuffer::new(
      renderer,
      mx_total_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );
    Self {
      gpu_camera_position,
      gpu_camera_position_dirty: false,
      gpu_mvp_matrix,
      gpu_mvp_matrix_dirty: false,
    }
  }

  pub fn mark_dirty(&mut self) {
    self.gpu_mvp_matrix_dirty = true;
    self.gpu_camera_position_dirty = true;
  }

  fn get_world_position_data(camera: &impl Camera) -> &[f32; 3] {
    let transform = camera.get_transform();
    transform.position.as_ref()
  }

  pub fn update_gpu_world_position(
    &mut self,
    renderer: &mut WGPURenderer,
    camera: &impl Camera,
  ) -> &WGPUBuffer {
    if !self.gpu_camera_position_dirty {
      return &self.gpu_camera_position;
    }
    self.gpu_camera_position_dirty = false;
    self
      .gpu_camera_position
      .update(renderer, CameraGPU::get_world_position_data(camera));
    &self.gpu_camera_position
  }

  // fn get_mvp_matrix_data(camera: &PerspectiveCamera) -> &[f32; 16] {
  //   let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();
  //   mx_total.as_ref()
  // }

  pub fn update_gpu_mvp_matrix(
    &mut self,
    renderer: &mut WGPURenderer,
    camera: &impl Camera,
  ) -> &WGPUBuffer {
    if !self.gpu_mvp_matrix_dirty {
      return &self.gpu_mvp_matrix;
    }
    self.gpu_mvp_matrix_dirty = false;

    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();
    let mx_total_ref: &[f32; 16] = mx_total.as_ref();

    self.gpu_mvp_matrix.update(renderer, mx_total_ref);
    &self.gpu_mvp_matrix
  }

  pub fn update_all(&mut self, renderer: &mut WGPURenderer, camera: &impl Camera) {
    self.update_gpu_mvp_matrix(renderer, camera);
    self.update_gpu_world_position(renderer, camera);
  }
}

impl GPUItem<PerspectiveCamera> for WGPUBuffer {
  fn create_gpu(item: &PerspectiveCamera, renderer: &mut WGPURenderer) -> Self {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();

    WGPUBuffer::new(
      renderer,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    )
  }
  fn update_gpu(&mut self, item: &PerspectiveCamera, renderer: &mut WGPURenderer) {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    self.update(renderer, mx_ref);
  }
}

impl GPUItem<ViewFrustumOrthographicCamera> for WGPUBuffer {
  fn create_gpu(item: &ViewFrustumOrthographicCamera, renderer: &mut WGPURenderer) -> Self {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();

    WGPUBuffer::new(
      renderer,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    )
  }
  fn update_gpu(&mut self, item: &ViewFrustumOrthographicCamera, renderer: &mut WGPURenderer) {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    self.update(renderer, mx_ref);
  }
}

impl GPUItem<ImageBuffer<Rgba<u8>, Vec<u8>>> for WGPUTexture {
  fn create_gpu(image: &ImageBuffer<Rgba<u8>, Vec<u8>>, renderer: &mut WGPURenderer) -> Self {
    WGPUTexture::new_from_image_data(
      renderer,
      &image.clone().into_raw(),
      (image.width(), image.height(), 1),
    )
  }
  fn update_gpu(&mut self, image: &ImageBuffer<Rgba<u8>, Vec<u8>>, renderer: &mut WGPURenderer) {
    todo!()
  }
}

pub fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
  Vertex {
    position: Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
    normal: Vec3::new(0.0, 1.0, 0.0),
    uv: Vec2::new(tc[0] as f32, tc[1] as f32),
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
