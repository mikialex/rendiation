//! The whole idea of extensible rendering architecture works like this:
//!
//! ```rust
//! fn demo_render() {
//!   let resource = create_reactive_gpu_resource_when_application_init();
//!   for frame in each_frame {
//!     // business_logic
//!     user_modify_scene_at_will();
//!
//!     let render_impl = resource.maintain_and_create_render_impl();
//!
//!     for pass in effects {
//!       render_impl.render(frame, pass)
//!     }
//!   }
//! }
//! ```

use database::*;
use reactive::*;
use rendiation_algebra::*;
use rendiation_scene_core::*;
use rendiation_texture_gpu_system::*;
use rendiation_webgpu::*;

mod light;
pub use light::*;
mod texture;
pub use texture::*;

pub trait RenderImplProvider<T> {
  /// this will be called once when application init
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU);
  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> T;
}

pub type GPUTextureBindingSystem = Box<dyn DynAbstractGPUTextureSystem>;

/// abstract over direct or indirect rendering
pub trait SceneRenderer: SceneModelRenderer {
  /// render all content in given scene.
  ///
  /// The rendering content is specified by implementation.
  /// The rendering content is refer to which models to draw, and in which order, or if draw background.
  /// The rendering may initialize multiple render pass and any encoder operation.
  ///
  /// the implementation may call `render_reorderable_models` internally. And the `pass` ctx should be
  /// passed to the internal call
  fn make_pass_content<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a>;

  /// return if requires clear. this supposed to be true when background is drawn, or directly as a way to impl
  /// solid background.
  fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>);

  fn get_scene_model_cx(&self) -> &GPUTextureBindingSystem;

  /// batch rendering passed models, the implementation is free to reorder the models in any ways.
  fn render_reorderable_models(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  );

  /// expose the underlayer camera system impl to enable user access the
  /// direct camera gpu manipulation, this is useful when some effect pipeline
  /// requires camera manipulation such as TAA.
  fn get_camera_gpu(&self) -> &dyn CameraRenderImpl;
}

pub trait CameraRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn make_dep_component(
    &self,
    idx: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderDependencyComponent + '_>>;

  fn setup_camera_jitter(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
    jitter: Vec2<f32>,
    queue: &GPUQueue,
  );
}

pub trait LightsRenderImpl {
  /// todo, in current impl, the lighting is truly global. todo: support filter by scene
  ///
  /// impl scene filter is complex to impl because the multi access indirect data required to be
  /// incrementally maintained in device
  fn make_component(&self) -> Option<Box<dyn RenderComponent + '_>>;
}

/// ability to do scene model level rendering
pub trait SceneModelRenderer {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &'a (dyn CameraRenderImpl + 'a),
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)>;

  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &dyn CameraRenderImpl,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) {
    if let Some((com, command)) = self.make_component(idx, camera, camera_gpu, pass, tex) {
      com.render(cx, command)
    }
  }

  /// maybe implementation could provide better performance for example host side multi draw
  fn render_reorderable_models_impl(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &dyn CameraRenderImpl,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) {
    for m in models {
      self.render_scene_model(m, camera, camera_gpu, pass, cx, tex);
    }
  }
}

impl SceneModelRenderer for Vec<Box<dyn SceneModelRenderer>> {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &'a (dyn CameraRenderImpl + 'a),
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    for provider in self {
      if let Some(com) = provider.make_component(idx, camera, camera_gpu, pass, tex) {
        return Some(com);
      }
    }
    None
  }
}
