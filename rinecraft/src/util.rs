use image::Rgba;
use crate::watch::GPUItem;
use rendiation::consts::OPENGL_TO_WGPU_MATRIX;
use rendiation::*;
use rendiation_render_entity::*;
use rendiation_math::{Vec2, Vec3};
use image::ImageBuffer;

impl GPUItem<PerspectiveCamera> for WGPUBuffer {
  fn create_gpu(item: &PerspectiveCamera, renderer: &mut WGPURenderer) -> Self {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();

    WGPUBuffer::new(
      &renderer.device,
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
      &renderer.device,
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
      &renderer.device, 
      &mut renderer.encoder, 
      &image.clone().into_raw(),
      (image.width(), image.height(), 1)
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


pub fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
  let vertex_data = [
    // top (0, 0, 1)
    vertex([-1, -1, 1], [0, 0]),
    vertex([1, -1, 1], [1, 0]),
    vertex([1, 1, 1], [1, 1]),
    vertex([-1, 1, 1], [0, 1]),
    // bottom (0, 0, -1)
    vertex([-1, 1, -1], [1, 0]),
    vertex([1, 1, -1], [0, 0]),
    vertex([1, -1, -1], [0, 1]),
    vertex([-1, -1, -1], [1, 1]),
    // right (1, 0, 0)
    vertex([1, -1, -1], [0, 0]),
    vertex([1, 1, -1], [1, 0]),
    vertex([1, 1, 1], [1, 1]),
    vertex([1, -1, 1], [0, 1]),
    // left (-1, 0, 0)
    vertex([-1, -1, 1], [1, 0]),
    vertex([-1, 1, 1], [0, 0]),
    vertex([-1, 1, -1], [0, 1]),
    vertex([-1, -1, -1], [1, 1]),
    // front (0, 1, 0)
    vertex([1, 1, -1], [1, 0]),
    vertex([-1, 1, -1], [0, 0]),
    vertex([-1, 1, 1], [0, 1]),
    vertex([1, 1, 1], [1, 1]),
    // back (0, -1, 0)
    vertex([1, -1, 1], [0, 0]),
    vertex([-1, -1, 1], [1, 0]),
    vertex([-1, -1, -1], [1, 1]),
    vertex([1, -1, -1], [0, 1]),
  ];

  let index_data: &[u16] = &[
    0, 1, 2, 2, 3, 0, // top
    4, 5, 6, 6, 7, 4, // bottom
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // front
    20, 21, 22, 22, 23, 20, // back
  ];

  (vertex_data.to_vec(), index_data.to_vec())
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
