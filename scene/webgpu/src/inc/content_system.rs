use __core::{
  pin::Pin,
  task::{Context, Poll},
};

use crate::*;
use std::sync::{Arc, RwLock};

/// The actual gpu data
///
/// we could customize the stream trait's context to avoid too much arc clone in update logic
#[pin_project::pin_project]
pub struct GlobalGPUSystem {
  gpu: ResourceGPUCtx,
  model_ctx: GPUModelResourceCtx,
  bindable_ctx: ShareBindableResourceCtx,
  texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureView>>>,
  // texture_cube: StreamMap<ReactiveGPUCubeTextureView>,
  // uniforms: HashMap<TypeId, Box<dyn Any>>,
  materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  // meshes: StreamMap<ReactiveRenderComponent>,
  #[pin]
  pub models: StreamMap<ModelGPUReactive>,
}

impl GlobalGPUSystem {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    let gpu = ResourceGPUCtx::new(gpu, mipmap_gen);

    let bindable_ctx = ShareBindableResourceCtx {
      gpu: gpu.clone(),
      texture_2d: Default::default(),
    };

    let model_ctx = GPUModelResourceCtx {
      shared: bindable_ctx.clone(),
      materials: Default::default(),
    };

    let texture_2d = bindable_ctx.texture_2d.clone();
    let materials = model_ctx.materials.clone();

    Self {
      gpu,
      bindable_ctx,
      model_ctx,
      texture_2d,
      materials,
      models: Default::default(),
    }
  }
}

impl Stream for GlobalGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    let mut texture_2d = this.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<ReactiveGPU2DTextureView> = &mut texture_2d;
    do_updates_by(texture_2d, cx, |_| {});

    let mut materials = this.materials.write().unwrap();
    let materials: &mut StreamMap<MaterialGPUInstance> = &mut materials;
    do_updates_by(materials, cx, |_| {});

    do_updates_by(&mut this.models, cx, |_| {});
    Poll::Pending
  }
}

#[derive(Clone)]
pub struct GPUModelResourceCtx {
  pub shared: ShareBindableResourceCtx,
  pub materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  // meshes: StreamMap<ReactiveRenderComponent>,
}

#[derive(Clone)]
pub struct ShareBindableResourceCtx {
  pub gpu: ResourceGPUCtx,
  pub texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureView>>>,
  // texture_cube:  mut StreamMap<ReactiveGPUCubeTextureView>,
  // uniforms:  mut HashMap<TypeId, Box<dyn Any>>,
}
