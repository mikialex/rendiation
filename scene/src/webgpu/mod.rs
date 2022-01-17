pub mod background;
pub mod bindgroup;
pub mod camera;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod rendering;
pub mod texture;

use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

pub use background::*;
pub use bindgroup::*;
pub use camera::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use rendering::*;
pub use texture::*;

use anymap::AnyMap;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig};
use rendiation_texture::TextureSampler;

use rendiation_webgpu::*;

use crate::{ResourceMapper, Scene, TextureCubeSource};

pub trait SceneRenderable: 'static {
  fn update(
    &self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtxBase,
    res: &mut GPUResourceSceneCache,
  );

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
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

#[derive(Default)]
pub struct GPUResourceCache {
  pub scene: GPUResourceSceneCache,
  pub content: GPUResourceSubCache,
}

#[derive(Default)]
pub struct GPUResourceSceneCache {
  pub materials: HashMap<TypeId, Box<dyn Any>>,
  pub meshes: HashMap<TypeId, Box<dyn Any>>,
}

/// GPU cache container for given scene
pub struct GPUResourceSubCache {
  pub cameras: CameraGPU,
  pub nodes: NodeGPU,
  pub texture_2ds: ResourceMapper<WebGPUTexture2d, Box<dyn WebGPUTexture2dSource>>,
  pub texture_cubes: ResourceMapper<WebGPUTextureCube, TextureCubeSource>,
  pub samplers: SamplerCache<TextureSampler>,
  pub pipeline_resource: PipelineResourceCache,
  pub layouts: BindGroupLayoutCache,
  pub custom_storage: AnyMap,
}

impl GPUResourceSubCache {
  pub fn maintain(&mut self) {
    self.cameras.maintain();
  }
}

impl Default for GPUResourceSubCache {
  fn default() -> Self {
    Self {
      texture_2ds: Default::default(),
      texture_cubes: Default::default(),
      cameras: Default::default(),
      samplers: Default::default(),
      pipeline_resource: Default::default(),
      layouts: Default::default(),
      custom_storage: AnyMap::new(),
      nodes: Default::default(),
    }
  }
}

impl Scene {
  pub fn create_material_ctx_base_and_models<'a>(
    &'a mut self,
    gpu: &GPU,
    pass_info: &'a RenderPassInfo,
    pass: &'a dyn PassDispatcher,
  ) -> (
    &'a mut GPUResourceSceneCache,
    SceneMaterialRenderPrepareCtxBase<'a>,
    &'a mut Vec<Box<dyn SceneRenderableRc>>,
  ) {
    let camera = self
      .active_camera
      .as_mut()
      .unwrap_or(&mut self.default_camera);
    self.resources.content.cameras.check_update_gpu(camera, gpu);

    (
      &mut self.resources.scene,
      SceneMaterialRenderPrepareCtxBase {
        camera,
        pass_info,
        resources: &mut self.resources.content,
        pass,
      },
      &mut self.models,
    )
  }

  pub fn create_material_ctx_base<'a>(
    &'a mut self,
    gpu: &GPU,
    pass_info: &'a RenderPassInfo,
    pass: &'a dyn PassDispatcher,
  ) -> (
    &'a mut GPUResourceSceneCache,
    SceneMaterialRenderPrepareCtxBase<'a>,
  ) {
    let (a, b, _) = self.create_material_ctx_base_and_models(gpu, pass_info, pass);
    (a, b)
  }
}
