#![feature(impl_trait_in_assoc_type)]

use std::{ops::DerefMut, sync::Arc};

use database::*;
use dyn_clone::*;
use fast_hash_collection::{FastHashMap, FastHashSet};
use parking_lot::RwLock;
use reactive::*;
use rendiation_algebra::*;
use rendiation_device_ray_tracing::*;
use rendiation_lighting_transport::DeviceSampler;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_texture_core::Size;
use rendiation_webgpu::*;
use tracing::*;

mod acceleration_structure;
pub use acceleration_structure::*;

mod material;
pub use material::*;

mod feature;
pub use feature::*;

mod bindless_mesh_bridge;
pub use bindless_mesh_bridge::*;

mod sampler;
pub use sampler::*;

mod sbt_util;
pub use sbt_util::*;

mod ray_util;
pub use ray_util::*;

mod camera;
pub use camera::*;

pub fn clamp_size_by_area(size: Size, area: usize) -> Size {
  assert!(area >= 1);
  let (width, height) = size.into_usize();
  let origin_area = width * height;
  let ratio = area as f32 / origin_area as f32;
  let ratio = ratio.sqrt();
  let width = (width as f32 * ratio).floor() as usize;
  let height = (height as f32 * ratio).floor() as usize;
  Size::from_usize_pair_min_one((width, height))
}

pub fn dispatch_size_depth_by(size: Size, depth: u32) -> (u32, u32, u32) {
  (size.width_usize() as u32, size.height_usize() as u32, depth)
}

pub fn dispatch_size(size: Size) -> (u32, u32, u32) {
  dispatch_size_depth_by(size, 1)
}
