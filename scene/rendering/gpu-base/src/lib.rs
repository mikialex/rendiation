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

use reactive::*;
use rendiation_scene_core::*;
use rendiation_webgpu::*;

pub trait RenderImplProvider<T> {
  /// this will be called once when application init
  fn register_resource(&mut self, source: &mut ReactiveStateJoinUpdater, cx: &GPUResourceCtx);
  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> T;
}

/// abstract over direct or indirect rendering
pub trait SceneRenderer: SceneModelRenderer {
  fn make_pass_content<'a>(
    &'a self,
    scene: AllocIdx<SceneEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a>;

  fn init_clear(
    &self,
    scene: AllocIdx<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>);
}

/// ability to do scene model level rendering
pub trait SceneModelRenderer {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &'a (dyn RenderComponent + 'a),
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)>;

  fn render_scene_model(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) {
    if let Some((com, command)) = self.make_component(idx, camera, pass) {
      com.render(cx, command)
    }
  }

  /// maybe implementation could provide better performance for example host side multi draw
  fn render_reorderable_models(
    &self,
    models: &mut dyn Iterator<Item = AllocIdx<SceneModelEntity>>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) {
    for m in models {
      self.render_scene_model(m, camera, pass, cx);
    }
  }
}

impl SceneModelRenderer for Vec<Box<dyn SceneModelRenderer>> {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &'a (dyn RenderComponent + 'a),
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    for provider in self {
      if let Some(com) = provider.make_component(idx, camera, pass) {
        return Some(com);
      }
    }
    None
  }
}
