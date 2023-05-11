#![feature(specialization)]
#![feature(hash_raw_entry)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
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
  collections::HashMap,
  hash::Hash,
  marker::PhantomData,
  rc::Rc,
  sync::{Arc, Mutex, RwLock},
};

use __core::hash::Hasher;
use __core::num::NonZeroU32;
use __core::{
  pin::Pin,
  task::{Context, Poll},
};
use anymap::AnyMap;
pub use background::*;
use bytemuck::*;
pub use camera::*;
use futures::*;
use incremental::*;
pub use lights::*;
use linked_hash_map::LinkedHashMap;
pub use materials::*;
pub use mesh::*;
pub use mipmap_gen::*;
pub use model::*;
pub use node::*;
use reactive::*;
pub use rendering::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_renderable_mesh::mesh::*;
pub use rendiation_scene_core::*;
use rendiation_texture::{Size, TextureSampler};
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
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
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
