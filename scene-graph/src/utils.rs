// use rendiation::*;
// use std::{ops::Range};
// use rendiation::renderer::{GPUGeometry};
// use rendiation::geometry::*;
// use crate::scene::resource::*;

// impl<T: PrimitiveTopology + 'static> Geometry for GPUGeometry<T> {
//   fn update_gpu(&mut self, renderer: &mut WGPURenderer) {
//     self.update_gpu(renderer)
//   }

//   fn get_gpu_index_buffer(&self) -> &WGPUBuffer {
//     self.get_index_buffer_unwrap()
//   }

//   fn get_gpu_vertex_buffer(&self, index: usize) -> &WGPUBuffer {
//     self.get_vertex_buffer_unwrap()
//   }

//   fn get_draw_range(&self) -> Range<u32> {
//     self.get_draw_range()
//   }
//   fn vertex_buffer_count(&self) -> usize {
//     1
//   }
// }
