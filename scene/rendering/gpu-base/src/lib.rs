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

use std::{
  any::{Any, TypeId},
  task::Context,
};

use fast_hash_collection::FastHashMap;
use reactive::*;
use rendiation_scene_core_next::*;
use rendiation_webgpu::*;

pub trait RenderImplProvider<T> {
  fn register_resource(&self, res: &mut ReactiveResourceManager);
  fn create_impl(&self, res: &ResourceUpdateResult) -> T;
}

pub trait SceneRenderer {
  fn render(&self, scene: AllocIdx<SceneEntity>) -> Box<dyn FrameContent>;
}

pub trait PassContentWithCamera {
  fn render(&mut self, pass: &mut FrameRenderPass, camera: AllocIdx<SceneCameraEntity>);
}

type BoxedFutureStream = Box<dyn Stream<Item = BoxedAnyFuture>>;
type BoxedAnyFuture = Box<dyn Future<Output = Box<dyn Any>>>;

#[derive(Default)]
pub struct ReactiveResourceManager {
  resource: Vec<BoxedFutureStream>,
  // waker: AtomicWaker,
}

impl ReactiveResourceManager {
  pub fn add_source_raw(&mut self, s: BoxedFutureStream) {
    self.resource.push(s);
    // self.waker.wake(); todo
  }
}

pub type ResourceUpdateResult = FastHashMap<TypeId, Box<dyn Any>>;

impl Stream for ReactiveResourceManager {
  type Item = ResourceUpdateResult;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    todo!()
  }
}
