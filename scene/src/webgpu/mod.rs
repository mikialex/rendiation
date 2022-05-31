pub mod background;
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

pub use background::*;
pub use camera::*;
// pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use rendering::*;
pub use shading::*;
pub use texture::*;

pub mod picking;
pub use picking::*;

use anymap::AnyMap;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig};

use rendiation_webgpu::*;

use crate::{IdentityMapper, Scene, SceneCamera, SceneContent, WebGPUScene};

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
  pub texture_2ds: IdentityMapper<GPUTexture2dView, <WebGPUScene as SceneContent>::Texture2D>,
  pub texture_cubes: IdentityMapper<GPUTextureCubeView, <WebGPUScene as SceneContent>::TextureCube>,
}

impl Scene<WebGPUScene> {
  pub fn add_model(&mut self, model: impl SceneRenderableShareable) {
    self.models.push(Box::new(model));
  }
}
