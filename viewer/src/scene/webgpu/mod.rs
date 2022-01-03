pub mod background;
pub mod bindgroup;
pub mod camera;
pub mod fatline;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod rendering;
pub mod texture;

pub use background::*;
pub use bindgroup::*;
pub use camera::*;
pub use fatline::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use rendering::*;
pub use texture::*;

use anymap::AnyMap;
use rendiation_texture::TextureSampler;

use rendiation_webgpu::*;

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
  pub(crate) samplers: SamplerCache<TextureSampler>,
  pub(crate) pipeline_resource: PipelineResourceCache,
  pub(crate) layouts: BindGroupLayoutCache,
  pub(crate) custom_storage: AnyMap,
}

impl Default for GPUResourceCache {
  fn default() -> Self {
    Self {
      samplers: Default::default(),
      pipeline_resource: Default::default(),
      layouts: Default::default(),
      custom_storage: AnyMap::new(),
    }
  }
}
