#![feature(specialization)]
#![feature(hash_raw_entry)]
#![feature(explicit_generic_args_with_impl_trait)]
#![allow(incomplete_features)]
#![allow(clippy::field_reassign_with_default)]

pub mod util;
pub use util::*;

pub use rendiation_scene_core::*;

pub mod background;
pub mod camera;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod rendering;
pub mod shading;
pub mod texture;

use __core::ops::Deref;
use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

use bytemuck::*;
use shadergraph::*;
use wgsl_shader_derives::*;

pub use background::*;
pub use camera::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use rendering::*;
pub use shading::*;
pub use texture::*;

use anymap::AnyMap;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig};

use webgpu::*;

use crate::{IdentityMapper, Scene, SceneCamera, SceneContent};

#[derive(Copy, Clone)]
pub struct WebGPUScene;
impl SceneContent for WebGPUScene {
  type BackGround = Box<dyn WebGPUBackground>;
  type Model = Box<dyn SceneRenderableShareable>;
  type Light = Box<dyn SceneRenderableShareable>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
  type SceneExt = ();
}

pub trait SceneRenderable: 'static {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  );

  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    None
  }
}

/// renderable but allow cheap clone and shared ownership
pub trait SceneRenderableShareable: SceneRenderable {
  fn id(&self) -> usize;
  fn clone_boxed(&self) -> Box<dyn SceneRenderableShareable>;
  fn as_renderable(&self) -> &dyn SceneRenderable;
  fn as_renderable_mut(&mut self) -> &mut dyn SceneRenderable;
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
  fn add_model(&mut self, model: impl SceneRenderableShareable);
  fn interaction_picking(
    &self,
    normalized_position: Vec2<f32>,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<&dyn SceneRenderableShareable>;
}

use std::cmp::Ordering;

impl WebGPUSceneExtension for Scene<WebGPUScene> {
  fn add_model(&mut self, model: impl SceneRenderableShareable) {
    self.models.push(Box::new(model));
  }

  fn interaction_picking(
    &self,
    normalized_position: Vec2<f32>,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<&dyn SceneRenderableShareable> {
    let mut result = Vec::new();

    let camera = self.active_camera.as_ref().unwrap();
    let world_ray = camera.cast_world_ray(normalized_position);

    for m in self.models.iter() {
      if let Some(Nearest(Some(r))) = m.ray_pick_nearest(&world_ray, conf) {
        println!("pick");
        result.push((m, r));
      }
    }

    result.sort_by(|(_, a), (_, b)| {
      a.hit
        .distance
        .partial_cmp(&b.hit.distance)
        .unwrap_or(Ordering::Less)
    });

    result.first().map(|r| r.0.as_ref())
  }
}
