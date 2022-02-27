// pub mod background;
pub mod bindgroup;
pub mod camera;
// pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod rendering;
pub mod shading;
pub mod texture;

use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

// pub use background::*;
pub use bindgroup::*;
pub use camera::*;
// pub use lights::*;
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

use rendiation_webgpu::*;

use crate::{ResourceMapper, SceneCamera, TextureCubeSource};

pub trait SceneRenderable: 'static {
  fn setup_pass(
    &self,
    gpu: &GPU,
    pass: &mut SceneRenderPass,
    camera_gpu: &SceneCamera,
    resources: &mut GPUResourceCache,
  );

  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    None
  }
}

pub trait SceneRenderableRc: SceneRenderable {
  fn id(&self) -> usize;
  fn clone_boxed(&self) -> Box<dyn SceneRenderableRc>;
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
  // pub uniforms: ResourceMapper<GPUTexture2d, Box<dyn WebGPUTexture2dSource>>,
  pub texture_2ds: ResourceMapper<GPUTexture2dView, Box<dyn WebGPUTexture2dSource>>,
  pub texture_cubes: ResourceMapper<GPUTextureCubeView, TextureCubeSource>,
}
