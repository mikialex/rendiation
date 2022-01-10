pub mod background;
pub mod bindgroup;
pub mod camera;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod rendering;
pub mod texture;

pub use background::*;
pub use bindgroup::*;
pub use camera::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use rendering::*;
pub use texture::*;

pub mod model_collection;
pub use model_collection::*;

use anymap::AnyMap;
use rendiation_texture::TextureSampler;

use rendiation_webgpu::*;

use crate::{ResourceMapper, TextureCubeSource};

pub trait SceneRenderable {
  fn update(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtxBase);

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  );
}

/// GPU cache container for given scene
///
/// Resources once allocate never release until the cache drop
pub struct GPUResourceCache {
  pub cameras: CameraGPU,
  pub nodes: NodeGPU,
  pub texture_2ds: ResourceMapper<WebGPUTexture2d, Box<dyn WebGPUTexture2dSource>>,
  pub texture_cubes: ResourceMapper<WebGPUTextureCube, TextureCubeSource>,
  pub samplers: SamplerCache<TextureSampler>,
  pub pipeline_resource: PipelineResourceCache,
  pub layouts: BindGroupLayoutCache,
  pub custom_storage: AnyMap,
}

impl GPUResourceCache {
  pub fn maintain(&mut self) {
    self.cameras.maintain();
  }
}

impl Default for GPUResourceCache {
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
