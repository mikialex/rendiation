pub mod forward;
pub use forward::*;
pub mod list;
pub use list::*;
pub mod copy_frame;
pub use copy_frame::*;
pub mod highlight;
pub use highlight::*;
pub mod background;
pub use background::*;
pub mod quad;
pub use quad::*;
pub mod framework;
pub use framework::*;
pub mod blur;
pub use blur::*;
pub mod defer;
pub use defer::*;
pub mod tonemap;
pub use tonemap::*;
pub mod debug_channels;
pub use debug_channels::*;
pub mod taa;
pub use taa::*;
pub mod ssao;
pub use ssao::*;
pub mod pass_base;
pub use pass_base::*;

use crate::*;

pub trait RenderComponent: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {
  fn render(&self, ctx: &mut GPURenderPassCtx, emitter: &dyn DrawcallEmitter) {
    let mut hasher = PipelineHasher::default();
    self.hash_pipeline(&mut hasher);

    let pipeline = ctx
      .gpu
      .device
      .get_or_cache_create_render_pipeline(hasher, |device| {
        device
          .build_pipeline_by_shadergraph(self.build_self().unwrap())
          .unwrap()
      });

    ctx.binding.reset();
    ctx.reset_vertex_binding_index();

    self.setup_pass_self(ctx);

    ctx.pass.set_pipeline_owned(&pipeline);

    ctx
      .binding
      .setup_pass(&mut ctx.pass, &ctx.gpu.device, &pipeline);

    emitter.draw(ctx);
  }
}

impl<T> RenderComponent for T where T: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {}

pub trait RenderComponentAny: RenderComponent + ShaderHashProviderAny {}
impl<T> RenderComponentAny for T where T: RenderComponent + ShaderHashProviderAny {}

pub trait DrawcallEmitter {
  fn draw(&self, ctx: &mut GPURenderPassCtx);
}

pub trait MeshDrawcallEmitter {
  fn draw(&self, ctx: &mut GPURenderPassCtx, group: MeshDrawGroup);
}

pub struct MeshDrawcallEmitterWrap<'a> {
  pub group: MeshDrawGroup,
  pub mesh: &'a dyn MeshDrawcallEmitter,
}

impl<'a> DrawcallEmitter for MeshDrawcallEmitterWrap<'a> {
  fn draw(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh.draw(ctx, self.group)
  }
}

pub struct RenderEmitter<'a, 'b> {
  contents: &'a [&'b dyn RenderComponentAny],
}

impl<'a, 'b> RenderEmitter<'a, 'b> {
  pub fn new(contents: &'a [&'b dyn RenderComponentAny]) -> Self {
    Self { contents }
  }
}

impl<'a, 'b> ShaderPassBuilder for RenderEmitter<'a, 'b> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.contents.iter().for_each(|c| c.setup_pass(ctx));
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self
      .contents
      .iter()
      .rev()
      .for_each(|c| c.post_setup_pass(ctx));
  }
}

impl<'a, 'b> ShaderHashProvider for RenderEmitter<'a, 'b> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .contents
      .iter()
      .for_each(|com| com.hash_pipeline_and_with_type_id(hasher))
  }
}

impl<'a, 'b> ShaderGraphProvider for RenderEmitter<'a, 'b> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for c in self.contents {
      c.build(builder)?;
    }
    Ok(())
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for c in self.contents.iter().rev() {
      c.post_build(builder)?;
    }
    Ok(())
  }
}

pub struct SceneRenderPass<'a, 'b, 'c> {
  pub ctx: GPURenderPassCtx<'a, 'b>,
  pub resources: &'c mut GPUResourceCache,
  pub pass_info: UniformBufferDataView<RenderPassGPUInfoData>,
}

impl<'a, 'b, 'c> SceneRenderPass<'a, 'b, 'c> {
  pub fn default_dispatcher(&self) -> DefaultPassDispatcher {
    DefaultPassDispatcher {
      formats: self.ctx.pass.formats().clone(),
      pass_info: self.pass_info.clone(),
      auto_write: true,
    }
  }
}

impl<'a, 'b, 'c> std::ops::Deref for SceneRenderPass<'a, 'b, 'c> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.ctx.pass
  }
}

impl<'a, 'b, 'c> std::ops::DerefMut for SceneRenderPass<'a, 'b, 'c> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.ctx.pass
  }
}

pub struct CameraRef<'a, T> {
  camera: &'a SceneCamera,
  inner: T,
}

impl<'a, T> CameraRef<'a, T> {
  pub fn with(camera: &'a SceneCamera, inner: T) -> Self {
    CameraRef { camera, inner }
  }
}

pub trait WebGPUScenePipelineHelper<S: SceneContent> {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T>;
  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T, S>;
}

impl<S: SceneContent> WebGPUScenePipelineHelper<S> for Scene<S> {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T> {
    CameraRef {
      camera: self.active_camera.as_ref().unwrap(),
      inner,
    }
  }

  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T, S> {
    CameraSceneRef {
      camera: self.active_camera.as_ref().unwrap(),
      scene: self,
      inner,
    }
  }
}

impl<'a, T: PassContentWithCamera> PassContent for CameraRef<'a, T> {
  fn render(&mut self, pass: &mut SceneRenderPass) {
    self.inner.render(pass, self.camera);
  }
}

pub trait PassContentWithCamera {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera);
}

pub trait PassContentWithSceneAndCamera<S: SceneContent> {
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera);
}

pub struct CameraSceneRef<'a, T, S: SceneContent> {
  pub camera: &'a SceneCamera,
  pub scene: &'a Scene<S>,
  pub inner: T,
}

impl<'a, T, S> PassContent for CameraSceneRef<'a, T, S>
where
  T: PassContentWithSceneAndCamera<S>,
  S: SceneContent,
{
  fn render(&mut self, pass: &mut SceneRenderPass) {
    self.inner.render(pass, self.scene, self.camera);
  }
}

pub trait RebuildAbleGPUCollectionBase {
  fn reset(&mut self);
  /// return count
  fn update_gpu(&mut self, gpu: &GPU) -> usize;
}
