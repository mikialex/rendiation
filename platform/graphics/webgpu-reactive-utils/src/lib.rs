use std::{
  marker::PhantomData,
  task::{Context, Poll},
};

use reactive_collection::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod storage;
pub use storage::*;
mod uniform;
pub use uniform::*;
