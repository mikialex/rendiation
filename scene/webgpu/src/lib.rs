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
pub use mipmap_gen::*;
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
use core::ops::Deref;
use futures::*;
use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  collections::HashMap,
  hash::Hash,
  marker::PhantomData,
  rc::Rc,
  sync::Mutex,
};

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
  pub texture_2ds: IdentityMapper<GPU2DTextureView, SceneTexture2DType>,
  pub texture_cubes: IdentityMapper<GPUCubeTextureView, SceneTextureCubeImpl>,
  pub mipmap_gen: Rc<RefCell<MipMapTaskManager>>,
}

pub trait WebGPUSceneExtension {
  fn build_interactive_ctx<'a>(
    &'a self,
    normalized_position: Vec2<f32>,
    camera_view_size: Size,
    conf: &'a MeshBufferIntersectConfig,
  ) -> SceneRayInteractiveCtx<'a>;

  fn interaction_picking<'a>(
    &'a self,
    ctx: &SceneRayInteractiveCtx,
    bounding_system: &mut SceneBoundingSystem,
  ) -> Option<(&'a SceneModel, MeshBufferHitPoint)>;
}

use std::cmp::Ordering;

impl WebGPUSceneExtension for SceneInner {
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

  fn interaction_picking<'a>(
    &'a self,
    ctx: &SceneRayInteractiveCtx,
    bounding_system: &mut SceneBoundingSystem,
  ) -> Option<(&'a SceneModel, MeshBufferHitPoint)> {
    bounding_system.maintain();
    interaction_picking(
      self.models.iter().filter_map(|(handle, m)| {
        if let Some(bounding) = bounding_system.get_model_bounding(handle) {
          if ctx.world_ray.intersect(bounding, &()) {
            Some(m)
          } else {
            println!("culled");
            None
          }
        } else {
          // unbound model
          Some(m)
        }
      }),
      ctx,
    )
  }
}

pub fn interaction_picking<'a, T: IntoIterator<Item = &'a SceneModel>>(
  content: T,
  ctx: &SceneRayInteractiveCtx,
) -> Option<(&'a SceneModel, MeshBufferHitPoint)> {
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

pub enum HitReaction {
  // AnyHit(MeshBufferHitPoint),
  Nearest(MeshBufferHitPoint),
  None,
}

pub fn interaction_picking_mut<
  'a,
  X: SceneRayInteractive + ?Sized + 'a,
  T: IntoIterator<Item = &'a mut X>,
>(
  content: T,
  ctx: &SceneRayInteractiveCtx,
  mut cb: impl FnMut(&'a mut X, HitReaction),
) {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(ctx) {
      // cb(m, HitReaction::AnyHit(r));
      result.push((m, r));
    } else {
      cb(m, HitReaction::None);
    }
  }

  result.sort_by(|(_, a), (_, b)| {
    a.hit
      .distance
      .partial_cmp(&b.hit.distance)
      .unwrap_or(Ordering::Less)
  });

  if let Some((m, r)) = result.into_iter().next() {
    cb(m, HitReaction::Nearest(r));
  }
}
