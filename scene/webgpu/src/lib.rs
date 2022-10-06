#![feature(specialization)]
#![feature(hash_raw_entry)]
#![allow(clippy::field_reassign_with_default)]
#![allow(incomplete_features)]

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
pub mod shadow;
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
pub use shadow::*;
pub use texture::*;
pub use util::*;

use anymap::AnyMap;
use bytemuck::*;
use linked_hash_map::LinkedHashMap;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_renderable_mesh::mesh::*;
pub use rendiation_scene_core::*;
use rendiation_texture::{CubeTextureFace, Size, TextureSampler};
use shadergraph::*;
use webgpu::*;
use wgsl_shader_derives::*;

use __core::hash::Hasher;
use __core::num::NonZeroU64;
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
  type Light = Box<dyn WebGPUSceneLight>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
  type SceneExt = ();
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

pub struct SceneRayInteractiveCtx<'a> {
  pub world_ray: Ray3,
  pub conf: &'a MeshBufferIntersectConfig,
  pub camera: &'a SceneCamera,
  pub camera_view_size: Size,
}

pub trait SceneRayInteractive {
  fn ray_pick_nearest(&self, _ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint>;
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
    self.nodes.maintain();
    self.content.texture_2ds.maintain();
    self.content.texture_cubes.maintain();
    // self.scene.lights
    todo!()
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
pub struct GPULightCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
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
  pub lights: GPULightCache,
  pub meshes: GPUMeshCache,
}

/// GPU cache container for given scene
#[derive(Default)]
pub struct GPUResourceSubCache {
  pub texture_2ds: IdentityMapper<GPUTexture2dView, dyn WebGPUTexture2dSource>,
  pub texture_cubes: IdentityMapper<GPUTextureCubeView, [Box<dyn WebGPUTexture2dSource>; 6]>,
}

use arena::Handle;
pub type SceneModelHandle = Handle<<WebGPUScene as SceneContent>::Model>;
pub type SceneCameraHandle = Handle<SceneCamera>;

pub trait WebGPUSceneExtension {
  #[must_use]
  fn add_model(&mut self, model: impl SceneModelShareable + 'static) -> SceneModelHandle;
  fn remove_model(&mut self, handle: SceneModelHandle) -> bool;
  #[must_use]
  fn add_camera(&mut self, camera: SceneCamera) -> SceneCameraHandle;
  fn remove_camera(&mut self, handle: SceneCameraHandle) -> bool;

  fn build_interactive_ctx<'a>(
    &'a self,
    normalized_position: Vec2<f32>,
    camera_view_size: Size,
    conf: &'a MeshBufferIntersectConfig,
  ) -> SceneRayInteractiveCtx<'a>;

  fn interaction_picking(
    &self,
    ctx: &SceneRayInteractiveCtx,
  ) -> Option<(&dyn SceneModelShareable, MeshBufferHitPoint)>;
}

use std::cmp::Ordering;

impl WebGPUSceneExtension for Scene<WebGPUScene> {
  fn add_model(&mut self, model: impl SceneModelShareable + 'static) -> SceneModelHandle {
    self.models.insert(Box::new(model))
  }
  fn remove_model(&mut self, handle: SceneModelHandle) -> bool {
    self.models.remove(handle).is_some()
  }
  fn add_camera(&mut self, camera: SceneCamera) -> SceneCameraHandle {
    self.cameras.insert(camera)
  }
  fn remove_camera(&mut self, handle: SceneCameraHandle) -> bool {
    self.cameras.remove(handle).is_some()
  }

  fn build_interactive_ctx<'a>(
    &'a self,
    normalized_position: Vec2<f32>,
    camera_view_size: Size,
    conf: &'a MeshBufferIntersectConfig,
  ) -> SceneRayInteractiveCtx<'a> {
    let camera = self.active_camera.as_ref().unwrap();
    let world_ray = camera.cast_world_ray(normalized_position);
    SceneRayInteractiveCtx {
      world_ray,
      conf,
      camera,
      camera_view_size,
    }
  }

  fn interaction_picking(
    &self,
    ctx: &SceneRayInteractiveCtx,
  ) -> Option<(&dyn SceneModelShareable, MeshBufferHitPoint)> {
    interaction_picking(self.models.iter().map(|(_, m)| m.as_ref()), ctx)
  }
}

impl<'a> SceneRayInteractive for &'a dyn SceneModelShareable {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self.as_interactive().ray_pick_nearest(ctx)
  }
}

pub fn interaction_picking<I: SceneRayInteractive, T: IntoIterator<Item = I>>(
  content: T,
  ctx: &SceneRayInteractiveCtx,
) -> Option<(I, MeshBufferHitPoint)> {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(ctx) {
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

pub fn interaction_picking_mut<
  'a,
  X: SceneRayInteractive + ?Sized,
  T: IntoIterator<Item = &'a mut X>,
>(
  content: T,
  ctx: &SceneRayInteractiveCtx,
  mut on_not_hit: impl FnMut(&'a mut X),
) -> Option<(&'a mut X, MeshBufferHitPoint)> {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(ctx) {
      result.push((m, r));
    } else {
      on_not_hit(m)
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
