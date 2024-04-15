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
  fn render(
    &self,
    scene: AllocIdx<SceneEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponentAny,
    ctx: &mut FrameCtx,
    target: RenderPassDescriptorOwned,
  );
}

pub trait PassContentWithCamera {
  fn render(&mut self, pass: &mut FrameRenderPass, camera: AllocIdx<SceneCameraEntity>);
}

type BoxedFutureStream = Box<dyn Stream<Item = BoxedAnyFuture>>;
type BoxedAnyFuture = Box<dyn Future<Output = Box<dyn Any>>>;

pub struct ReactiveResourceManager {
  resource: FastHashMap<TypeId, BoxedFutureStream>,
  cx: GPUResourceCtx,
}

impl ReactiveResourceManager {
  pub fn cx(&self) -> &GPUResourceCtx {
    &self.cx
  }

  pub fn register_source_raw(&mut self, id: TypeId, s: BoxedFutureStream) {
    self.resource.insert(id, s);
  }

  pub fn register_multi_updater<T: 'static>(&mut self, updater: MultiUpdateContainer<T>) {
    let updater = Box::new(SharedMultiUpdateContainer::new(updater)) as BoxedFutureStream;
    self.register_source_raw(TypeId::of::<MultiUpdateContainer<T>>(), updater);
  }
}

pub struct ResourceUpdateResult {
  inner: FastHashMap<TypeId, Box<dyn Any>>,
}

impl ResourceUpdateResult {
  pub fn get_multi_updater<T>(&self) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    let t = TypeId::of::<MultiUpdateContainer<T>>();
    self
      .inner
      .get(&t)?
      .downcast_ref::<LockReadGuardHolder<MultiUpdateContainer<T>>>()?
      .clone()
      .into()
  }
}

impl Stream for ReactiveResourceManager {
  type Item = ResourceUpdateResult;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    // join_all(
    //   self
    //     .get_mut()
    //     .resource
    //     .values_mut()
    //     .map(|v| v.poll_next(cx)),
    // );
    todo!()
  }
}
