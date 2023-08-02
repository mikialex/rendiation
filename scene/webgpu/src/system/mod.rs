use crate::*;

mod wrapper;
pub use wrapper::*;

mod content_system;
pub use content_system::*;

mod scene_system;
pub use scene_system::*;

mod global_system;
pub use global_system::*;

#[derive(Clone)]
pub struct ResourceGPUCtx {
  pub device: GPUDevice,
  pub queue: GPUQueue,
  pub mipmap_gen: Rc<RefCell<MipMapTaskManager>>,
}

impl ResourceGPUCtx {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    Self {
      device: gpu.device.clone(),
      queue: gpu.queue.clone(),
      mipmap_gen,
    }
  }
}

pub struct BindlessResourceProvider<'a, T> {
  pub(crate) base: &'a T,
  pub(crate) texture_system: &'a WebGPUTextureBindingSystem,
}

impl<'a, T: ShaderPassBuilder> ShaderPassBuilder for BindlessResourceProvider<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.setup_pass(ctx);
    self.texture_system.setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.post_setup_pass(ctx)
  }
}

impl<'a, T: ShaderHashProviderAny> ShaderHashProviderAny for BindlessResourceProvider<'a, T> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    struct Marker;
    Marker.type_id().hash(hasher);
    self.base.hash_pipeline_and_with_type_id(hasher)
  }
}

impl<'a, T: ShaderHashProvider> ShaderHashProvider for BindlessResourceProvider<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.base.hash_pipeline(hasher);
    self.texture_system.hash_pipeline(hasher)
  }
}
impl<'a, T: ShaderGraphProvider> ShaderGraphProvider for BindlessResourceProvider<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.base.build(builder)?;
    self.texture_system.build(builder)
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.base.post_build(builder)
  }
}
