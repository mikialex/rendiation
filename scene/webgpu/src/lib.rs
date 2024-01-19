#![feature(specialization)]
#![feature(let_chains)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod background;
mod camera;

mod lights;
mod materials;
mod mesh;
mod model;
mod node;
mod rendering;
mod texture;

mod system;
use core::ops::Deref;
use std::{
  any::{Any, TypeId},
  cell::RefCell,
  hash::Hash,
  rc::Rc,
  sync::{Arc, RwLock},
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
use rendiation_texture_gpu_base::*;
use rendiation_texture_gpu_system::*;
use rendiation_webgpu::*;
pub use system::*;
pub use texture::*;

pub trait SceneRenderable {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  );
}
