//! ```rust
//! fn demo_render() {
//!   let resource = create_reactive_gpu_resource_when_application_init();
//!   for frame in each_frame {
//!     // business_logic
//!     user_modify_scene_at_will();
//!
//!     resource.maintain_on_demand();
//!     let render_impl = resource.create_render_impl();
//!     for pass in effects {
//!       for scene_pass_content in scene_pass_content_split {
//!         pass.setup(scene_pass_content)
//!         // for example if the gles scene_pass_content then:
//!         // for single_dispatch in scene {
//!         //   render_impl.render(model, pass)
//!         // }
//!       }
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

pub trait RenderImplProvider<T> {
  /// this will be called once when application init
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU);
  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> T;
}

pub type GPUTextureBindingSystem = Box<dyn DynAbstractGPUTextureSystem>;

/// abstract over direct or indirect rendering
pub trait SceneRenderer: SceneModelRenderer {
  fn make_pass_content<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a>;

  fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>);

  fn get_scene_model_cx(&self) -> &GPUTextureBindingSystem;

  fn render_reorderable_models(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  );

  fn get_camera_gpu(&self) -> &dyn GLESCameraRenderImpl;
}

pub trait GLESCameraRenderImpl {
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

/// ability to do scene model level rendering
pub trait SceneModelRenderer {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &'a (dyn GLESCameraRenderImpl + 'a),
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)>;

  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &dyn GLESCameraRenderImpl,
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
    camera_gpu: &dyn GLESCameraRenderImpl,
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
    camera_gpu: &'a (dyn GLESCameraRenderImpl + 'a),
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
