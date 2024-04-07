use std::{
  marker::PhantomData,
  task::{Context, Poll},
};

use reactive::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod storage;
pub use storage::*;
mod uniform;
pub use uniform::*;
mod cube_map;
pub use cube_map::*;
