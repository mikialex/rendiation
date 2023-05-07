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
pub mod model_overrides;
pub mod node;
pub mod picking;
pub mod rendering;
pub mod shading;
pub mod shadow;
pub mod texture;
pub mod util;

mod system;
pub use system::*;

pub use background::*;
pub use camera::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use mipmap_gen::*;
pub use model::*;
pub use model_overrides::*;
pub use node::*;
pub use picking::*;
pub use rendering::*;
pub use shading::*;
pub use shadow::*;
pub use texture::*;
pub use util::*;

use anymap::AnyMap;
use bytemuck::*;
use incremental::*;
use linked_hash_map::LinkedHashMap;
use reactive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_renderable_mesh::mesh::*;
pub use rendiation_scene_core::*;
use rendiation_texture::{Size, TextureSampler};
use shadergraph::*;
use webgpu::*;
use wgsl_shader_derives::*;

use __core::hash::Hasher;
use __core::{
  pin::Pin,
  task::{Context, Poll},
};
use core::ops::Deref;
use futures::*;
use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  collections::HashMap,
  hash::Hash,
  marker::PhantomData,
  rc::Rc,
  sync::{Arc, Mutex, RwLock},
};

pub fn register_webgpu_extra_features() {
  register_core_material_features::<SceneItemRef<FatLineMaterial>>();
  register_webgpu_material_features::<SceneItemRef<FatLineMaterial>>();
  register_core_mesh_features::<SceneItemRef<FatlineMesh>>();
  register_webgpu_mesh_features::<SceneItemRef<FatlineMesh>>();
}

pub trait SceneRenderable {
  fn is_transparent(&self) -> bool {
    false
  }

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

/// renderable but allow cheap clone and shared ownership
pub trait SceneRenderableShareable: SceneRenderable {
  fn id(&self) -> usize;
  fn clone_boxed(&self) -> Box<dyn SceneRenderableShareable>;
  fn as_renderable(&self) -> &dyn SceneRenderable;
  fn as_renderable_mut(&mut self) -> &mut dyn SceneRenderable;
}

impl SceneRenderable for Box<dyn SceneRenderableShareable> {
  fn is_transparent(&self) -> bool {
    self.as_ref().is_transparent()
  }
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.as_ref().render(pass, dispatcher, camera)
  }
}
