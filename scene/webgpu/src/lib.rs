#![feature(specialization)]
#![feature(let_chains)]
#![feature(hash_raw_entry)]
#![feature(type_name_of_val)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(clippy::field_reassign_with_default)]
#![allow(incomplete_features)]

mod background;
mod camera;

mod lights;
mod materials;
mod mesh;
mod model;
mod node;
mod rendering;
mod shading;
mod texture;
mod util;

mod system;
use core::ops::Deref;
use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  hash::Hash,
  marker::PhantomData,
  rc::Rc,
  sync::{Arc, Mutex, RwLock},
};

use __core::{
  pin::Pin,
  task::{Context, Poll},
};
use anymap::AnyMap;
pub use background::*;
use bytemuck::*;
pub use camera::*;
use fast_hash_collection::*;
use futures::stream::FusedStream;
use futures::*;
use incremental::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
use reactive::*;
pub use rendering::*;
use rendiation_algebra::*;
use rendiation_lighting_transport::*;
use rendiation_mesh_core::*;
use rendiation_mesh_gpu_system::*;
use rendiation_scene_core::*;
use rendiation_shader_api::*;
use rendiation_texture::TextureSampler;
use rendiation_texture_gpu_system::*;
use rendiation_webgpu::*;
pub use shading::*;
pub use system::*;
pub use texture::*;
pub use util::*;

pub trait SceneRenderable {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  );
}
