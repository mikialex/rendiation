#![feature(associated_type_defaults)]

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
use std::{hash::Hash, mem::ManuallyDrop};

use database::*;
use dyn_clone::*;
use reactive::*;
use rendiation_algebra::*;
use rendiation_color::*;
use rendiation_device_parallel_compute::*;
use rendiation_scene_core::*;
use rendiation_shader_api::*;
use rendiation_texture_core::*;
use rendiation_texture_gpu_base::*;
use rendiation_texture_gpu_system::*;
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod alpha;
pub use alpha::*;
mod device_culling;
pub use device_culling::*;
mod camera;
pub use camera::*;
mod light;
pub use light::*;
mod texture;
pub use texture::*;
mod background;
pub use background::*;
mod batch;
pub use batch::*;
mod mid;
pub use mid::*;

both!(LogicalRenderEntityId, u32);

pub struct PbrMRMaterialTag;
pub struct PbrSGMaterialTag;

/// All color in shader should be in linear space, for some scene API that use sRGB color space, use this to convert before upload the
/// data into the gpu.
pub fn srgb4_to_linear4(color: Vec4<f32>) -> Vec4<f32> {
  let linear = LinearRGBColor::from(SRGBColor::from(color.xyz()));
  Vec4::new(linear.r, linear.g, linear.b, color.w)
}
pub fn srgb3_to_linear3(color: Vec3<f32>) -> Vec3<f32> {
  let linear = LinearRGBColor::from(SRGBColor::from(color.xyz()));
  Vec3::new(linear.r, linear.g, linear.b)
}

pub type BoxedQueryBasedGPUFeature<T> = Box<dyn QueryBasedFeature<T, Context = GPU>>;
pub trait QueryBasedFeature<T> {
  type Context;
  /// this will be called once when application init
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, ctx: &Self::Context);
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx);
  fn create_impl(&self, cx: &mut QueryResultCtx) -> T;
}

pub enum CameraRenderSource {
  Scene(EntityHandle<SceneCameraEntity>),
  External(Box<dyn RenderComponent>),
}

pub type GPUTextureBindingSystem = Box<dyn DynAbstractGPUTextureSystem>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneContentKey {
  pub only_alpha_blend_objects: Option<bool>,
}

/// A scene renderer that encapsulate the scene rendering ability.
pub trait SceneRenderer: SceneModelRenderer {
  /// A user defined content semantic key. This key is used to represent the semantic part of the scene content.
  /// These content is the scene's user-defined internal structure that require different pass effects or drawn in given order.
  type ContentKey = SceneContentKey;

  /// extract batched scene model by given content semantic, the extracted batch may be used by external
  /// system for further processing, for example culling. the simple culling logic may also be implemented here
  #[must_use]
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: Self::ContentKey,
    ctx: &mut FrameCtx,
  ) -> SceneModelRenderBatch;

  /// render batched scene model with given pass component on given pass
  #[must_use]
  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: CameraRenderSource,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a>;

  #[must_use]
  fn extract_and_make_pass_content<'a>(
    &'a self,
    semantic: Self::ContentKey,
    scene: EntityHandle<SceneEntity>,
    camera: CameraRenderSource,
    ctx: &mut FrameCtx,
    pass: &'a dyn RenderComponent,
  ) -> Box<dyn PassContent + 'a> {
    let batch = self.extract_scene_batch(scene, semantic, ctx);
    self.make_scene_batch_pass_content(batch, camera, pass, ctx)
  }

  /// Return if the scene rendering requires clear in every sub draw pass.
  /// This is supposed to be false when background is drawn by custom object.
  /// The simple solid background could use this as the implementation
  fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>);

  fn render_background<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: CameraRenderSource,
    tonemap: &'a dyn RenderComponent,
  ) -> Box<dyn PassContent + 'a>;

  /// Batch rendering the passed models. Comparing to render one single model at a time(using [SceneModelRenderer]), this may be more efficient.
  /// The implementation should be override if it can provide better performance. The default implementation is a loop call using [SceneModelRenderer]
  #[must_use]
  fn render_models<'a>(
    &'a self,
    models: Box<dyn HostRenderBatch>,
    camera: CameraRenderSource,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let batch = SceneModelRenderBatch::Host(models);
    self.make_scene_batch_pass_content(batch, camera, pass, ctx)
  }

  /// Expose the under-layer camera system implementation to enable user access the
  /// direct camera gpu manipulation(for example access and modify the camera gpu data).
  /// This is useful for some effect pipelines that require directly camera manipulation for example TAA.
  fn get_camera_gpu(&self) -> &dyn CameraRenderImpl;
}

/// A renderer supports rendering in scene model granularity
pub trait SceneModelRenderer {
  /// return if render successfully
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError>;
}

#[derive(thiserror::Error, Debug)]
pub enum UnableToRenderSceneModelError {
  #[error("failed to find model renderer impl for: {model_id} the sub tries are: {tried:?}")]
  UnableToFindImpl {
    model_id: EntityHandle<SceneModelEntity>,
    tried: Vec<Self>,
  },
  #[error("model renderer impl found but unable to render, the detail is: {0}")]
  FoundImplButUnableToRender(#[from] Box<dyn std::error::Error>),
}

impl SceneModelRenderer for Vec<Box<dyn SceneModelRenderer>> {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    for r in self {
      if r.render_scene_model(idx, camera, pass, cx, tex).is_ok() {
        return Ok(());
      }
    }
    let tried = self
      .iter()
      .map(|v| {
        v.render_scene_model(idx, camera, pass, cx, tex)
          .unwrap_err()
      })
      .collect();

    Err(UnableToRenderSceneModelError::UnableToFindImpl {
      model_id: idx,
      tried,
    })
  }
}

pub trait FrameCtxParallelComputeExt {
  fn access_parallel_compute<R>(&mut self, f: impl FnOnce(&mut DeviceParallelComputeCtx) -> R)
    -> R;
}

impl FrameCtxParallelComputeExt for FrameCtx<'_> {
  fn access_parallel_compute<R>(
    &mut self,
    f: impl FnOnce(&mut DeviceParallelComputeCtx) -> R,
  ) -> R {
    let mut ctx = DeviceParallelComputeCtx::new(self.gpu, &mut self.encoder);
    let r = f(&mut ctx);
    ctx.flush_pass();
    let _ = ManuallyDrop::new(ctx); // avoid drop to avoid unnecessary submit
    r
  }
}
