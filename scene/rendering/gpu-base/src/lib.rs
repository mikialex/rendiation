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
  fn register_resource(&mut self, source: &mut ConcurrentStreamContainer, cx: &GPUResourceCtx);
  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> T;
}

pub trait SceneRenderer {
  fn render(
    &self,
    scene: AllocIdx<SceneEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    ctx: &mut FrameCtx,
    target: RenderPassDescriptorOwned,
  );
}

pub trait SceneRasterRenderingAdaptor {
  type DrawTask;

  /// should contains frustum culling and lod select
  fn create_task(
    camera: AllocIdx<SceneCameraEntity>,
    scene: AllocIdx<SceneEntity>,
  ) -> Self::DrawTask;

  fn render_task_on_frame(&self, ctx: &mut FrameCtx, task: Self::DrawTask, target: &Attachment);
}

pub trait PassContentWithCamera {
  fn render(&mut self, pass: &mut FrameRenderPass, camera: AllocIdx<SceneCameraEntity>);
}
