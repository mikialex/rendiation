use image::ImageBuffer;
use image::Rgba;
use rendiation_algebra::{Mat4, Vec2, Vec3};
use rendiation_ral::ResourceManager;
use rendiation_render_entity::*;
use rendiation_scenegraph::{Scene, UniformHandle};
use rendiation_shader_library::transform::CameraTransform;
use rendiation_webgpu::consts::OPENGL_TO_WGPU_MATRIX;
use rendiation_webgpu::*;

// pub struct CameraGPU {
//   pub gpu_mvp_matrix: UniformHandle<WebGPU, CameraTransform>,
// }

// impl CameraGPU {
//   pub fn new(
//     renderer: &WGPURenderer,
//     camera: &Camera,
//     resources: &mut ResourceManager<WebGPU>,
//   ) -> Self {
//     let mvp = CameraTransform {
//       mvp: OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix(),
//       projection: OPENGL_TO_WGPU_MATRIX * *camera.get_projection_matrix(),
//       model_view: camera.get_view_matrix(),
//     };
//     Self {
//       gpu_mvp_matrix: resources.bindable.uniform_buffers.add(mvp),
//     }
//   }

//   pub fn update_gpu_mvp_matrix(
//     &mut self,
//     renderer: &mut WGPURenderer,
//     camera: &Camera,
//     resources: &mut ResourceManager<WebGPU>,
//   ) {
//     let mvp = CameraTransform {
//       mvp: OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix(),
//       projection: OPENGL_TO_WGPU_MATRIX * *camera.get_projection_matrix(),
//       model_view: camera.get_view_matrix(),
//     };

//     resources
//       .bindable
//       .uniform_buffers
//       .update(self.gpu_mvp_matrix, mvp);
//   }

//   pub fn update_all(
//     &mut self,
//     camera: &Camera,
//     renderer: &mut WGPURenderer,
//     resources: &mut ResourceManager<WebGPU>,
//   ) {
//     self.update_gpu_mvp_matrix(renderer, camera, resources);
//   }
// }

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
