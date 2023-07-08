#![feature(specialization)]
#![feature(hash_raw_entry)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(clippy::field_reassign_with_default)]
#![allow(incomplete_features)]

pub mod background;
pub mod camera;
pub mod lights;
pub mod materials;
pub mod mesh;
mod mipmap_gen;
pub mod model;
pub mod node;
pub mod rendering;
pub mod shading;
pub mod shadow;
pub mod texture;
pub mod util;

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
use futures::*;
use incremental::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use mipmap_gen::*;
pub use model::*;
pub use node::*;
use reactive::*;
pub use rendering::*;
use rendiation_algebra::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_renderable_mesh::mesh::*;
pub use rendiation_scene_core::*;
use rendiation_texture::TextureSampler;
use shadergraph::*;
pub use shading::*;
pub use shadow::*;
pub use system::*;
pub use texture::*;
pub use util::*;
use webgpu::*;
use wgsl_shader_derives::*;

pub trait SceneRenderable {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  );
}

pub trait SceneNodeControlled {
  fn visit_node(&self, visitor: &mut dyn FnMut(&SceneNode));
  fn get_node(&self) -> SceneNode {
    let mut result = None;
    self.visit_node(&mut |node| {
      result = node.clone().into();
    });
    result.unwrap()
  }
}
