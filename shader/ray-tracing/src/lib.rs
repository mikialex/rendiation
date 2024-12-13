#![feature(mapped_lock_guards)]

use std::any::Any;
use std::any::TypeId;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

use dyn_clone::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
use rendiation_algebra::*;
use rendiation_device_parallel_compute::*;
pub use rendiation_device_task_graph::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod api;
pub use api::*;

mod operator;
pub use operator::*;

mod backend;
pub use backend::*;

mod texture_io;
pub use texture_io::*;

#[cfg(test)]
mod test;

pub fn tiling_iter(full_size: (u32, u32), tile_size: u32) -> impl Iterator<Item = TiledUnit> {
  let x_repeat = full_size.0 / tile_size;
  let y_repeat = full_size.1 / tile_size;

  (0..x_repeat)
    .flat_map(move |x| (0..y_repeat).map(move |y| (x, y)))
    .map(move |(x, y)| {
      let offset = (x * tile_size, y * tile_size);
      let x_left = full_size.0 - offset.0;
      let y_left = full_size.1 - offset.1;

      TiledUnit {
        offset,
        size: (tile_size.min(x_left), tile_size.min(y_left)),
      }
    })
}

pub struct TiledUnit {
  pub offset: (u32, u32),
  pub size: (u32, u32), // size may smaller than tile_size
}
