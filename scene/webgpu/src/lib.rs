#![feature(min_specialization)]
#![feature(hash_raw_entry)]
#![allow(clippy::field_reassign_with_default)]

pub mod background;
pub mod camera;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod model_overrides;
pub mod node;
pub mod rendering;
pub mod shading;
pub mod texture;
pub mod util;

pub use background::*;
pub use camera::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use model_overrides::*;
pub use node::*;
pub use rendering::*;
pub use shading::*;
pub use texture::*;
pub use util::*;

use anymap::AnyMap;
use bytemuck::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_renderable_mesh::mesh::*;
pub use rendiation_scene_core::*;
use rendiation_texture::{CubeTextureFace, Size, TextureSampler};
use shadergraph::*;
use webgpu::util::DeviceExt;
use webgpu::*;
use wgsl_shader_derives::*;

use core::ops::Deref;
use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  collections::HashMap,
  hash::Hash,
  marker::PhantomData,
  rc::Rc,
  sync::Mutex,
};

#[derive(Copy, Clone)]
pub struct WebGPUScene;
impl SceneContent for WebGPUScene {
  type BackGround = Box<dyn WebGPUBackground>;
  type Model = Box<dyn SceneModelShareable>;
  type Light = Box<dyn SceneRenderableShareable>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
  type SceneExt = ();
}

pub trait SceneRenderable {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  );
}

pub trait SceneRayInteractive {
  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<MeshBufferHitPoint>;
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

pub trait SceneModelShareable:
  SceneRayInteractive + SceneRenderableShareable + SceneNodeControlled
{
  fn as_interactive(&self) -> &dyn SceneRayInteractive;
  fn as_renderable(&self) -> &dyn SceneRenderableShareable;
}
impl<T> SceneModelShareable for T
where
  T: SceneRayInteractive + SceneRenderableShareable + SceneNodeControlled,
{
  fn as_interactive(&self) -> &dyn SceneRayInteractive {
    self
  }
  fn as_renderable(&self) -> &dyn SceneRenderableShareable {
    self
  }
}
pub trait SceneModel: SceneRayInteractive + SceneRenderable + SceneNodeControlled {
  fn as_interactive(&self) -> &dyn SceneRayInteractive;
  fn as_renderable(&self) -> &dyn SceneRenderable;
}
impl<T> SceneModel for T
where
  T: SceneRayInteractive + SceneRenderable + SceneNodeControlled,
{
  fn as_interactive(&self) -> &dyn SceneRayInteractive {
    self
  }
  fn as_renderable(&self) -> &dyn SceneRenderable {
    self
  }
}

impl SceneRayInteractive for &mut dyn SceneModelShareable {
  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    todo!()
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
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.as_ref().render(pass, dispatcher, camera)
  }
}

pub struct GPUResourceCache {
  pub scene: GPUResourceSceneCache,
  pub content: GPUResourceSubCache,
  pub custom_storage: AnyMap,
  pub cameras: CameraGPUStore,
  pub nodes: NodeGPUStore,
}

impl GPUResourceCache {
  pub fn maintain(&mut self) {
    self.cameras.maintain();
  }
}

impl Default for GPUResourceCache {
  fn default() -> Self {
    Self {
      scene: Default::default(),
      content: Default::default(),
      custom_storage: AnyMap::new(),
      cameras: Default::default(),
      nodes: Default::default(),
    }
  }
}

#[derive(Default)]
pub struct GPUMaterialCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}
#[derive(Default)]
pub struct GPUMeshCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Default)]
pub struct GPUResourceSceneCache {
  pub materials: GPUMaterialCache,
  pub meshes: GPUMeshCache,
}

/// GPU cache container for given scene
#[derive(Default)]
pub struct GPUResourceSubCache {
  // pub uniforms: IdentityMapper<GPUTexture2d, Box<dyn WebGPUTexture2dSource>>,
  pub texture_2ds: IdentityMapper<GPUTexture2dView, dyn WebGPUTexture2dSource>,
  pub texture_cubes: IdentityMapper<GPUTextureCubeView, [Box<dyn WebGPUTexture2dSource>; 6]>,
}

pub trait WebGPUSceneExtension {
  fn add_model(&mut self, model: impl SceneModelShareable + 'static);
  fn build_picking_ray_by_view_camera(&self, normalized_position: Vec2<f32>) -> Ray3;
  fn interaction_picking(
    &self,
    normalized_position: Vec2<f32>,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<(&dyn SceneModelShareable, MeshBufferHitPoint)>;
}

use std::cmp::Ordering;

impl WebGPUSceneExtension for Scene<WebGPUScene> {
  fn add_model(&mut self, model: impl SceneModelShareable + 'static) {
    self.models.push(Box::new(model));
  }
  fn build_picking_ray_by_view_camera(&self, normalized_position: Vec2<f32>) -> Ray3 {
    let camera = self.active_camera.as_ref().unwrap();
    camera.cast_world_ray(normalized_position)
  }

  fn interaction_picking(
    &self,
    normalized_position: Vec2<f32>,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<(&dyn SceneModelShareable, MeshBufferHitPoint)> {
    let world_ray = self.build_picking_ray_by_view_camera(normalized_position);
    interaction_picking(self.models.iter().map(|m| m.as_ref()), world_ray, conf)
  }
}

impl<'a> SceneRayInteractive for &'a dyn SceneModelShareable {
  fn ray_pick_nearest(
    &self,
    world_ray: &Ray3,
    conf: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    self.as_interactive().ray_pick_nearest(world_ray, conf)
  }
}

pub fn interaction_picking<I: SceneRayInteractive, T: IntoIterator<Item = I>>(
  content: T,
  world_ray: Ray3,
  conf: &MeshBufferIntersectConfig,
) -> Option<(I, MeshBufferHitPoint)> {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(&world_ray, conf) {
      result.push((m, r));
    }
  }

  result.sort_by(|(_, a), (_, b)| {
    a.hit
      .distance
      .partial_cmp(&b.hit.distance)
      .unwrap_or(Ordering::Less)
  });

  result.into_iter().next()
}

pub fn interaction_picking_mut<'a, T: IntoIterator<Item = &'a mut dyn SceneRayInteractive>>(
  content: T,
  world_ray: Ray3,
  conf: &MeshBufferIntersectConfig,
) -> Option<(&'a mut dyn SceneRayInteractive, MeshBufferHitPoint)> {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(&world_ray, conf) {
      result.push((m, r));
    }
  }

  result.sort_by(|(_, a), (_, b)| {
    a.hit
      .distance
      .partial_cmp(&b.hit.distance)
      .unwrap_or(Ordering::Less)
  });

  result.into_iter().next()
}
