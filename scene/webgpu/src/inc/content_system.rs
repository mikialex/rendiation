use __core::{
  pin::Pin,
  task::{Context, Poll},
};

use crate::*;
use std::sync::{Arc, RwLock};

/// The actual gpu data
///
/// we could customize the stream trait's context to avoid too much arc clone in update logic
pub struct GlobalGPUSystem {
  gpu: ResourceGPUCtx,
  texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureView>>>,
  // texture_cube: StreamMap<ReactiveGPUCubeTextureView>,
  // uniforms: HashMap<TypeId, Box<dyn Any>>,
  materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  // meshes: StreamMap<ReactiveRenderComponent>,
  models: StreamMap<ModelGPUReactive>,
}

impl GlobalGPUSystem {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    let gpu = ResourceGPUCtx::new(gpu, mipmap_gen);
    Self {
      gpu,
      texture_2d: Default::default(),
      materials: Default::default(),
      models: Default::default(),
    }
  }
}

impl Stream for GlobalGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut texture_2d = self.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<ReactiveGPU2DTextureView> = &mut texture_2d;
    do_updates_by(texture_2d, cx, |_| {});

    let mut materials = self.materials.write().unwrap();
    let materials: &mut StreamMap<MaterialGPUInstance> = &mut materials;
    do_updates_by(materials, cx, |_| {});

    // do_updates(&mut self.models, |_| {});
    Poll::Pending
  }
}

#[derive(Clone)]
pub struct GlobalGPUSystemModelContentView {
  pub shared: ShareBindableResource,
  pub materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  // meshes: StreamMap<ReactiveRenderComponent>,
}

#[derive(Clone)]
pub struct ShareBindableResource {
  pub gpu: ResourceGPUCtx,
  pub texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureView>>>,
  // texture_cube:  mut StreamMap<ReactiveGPUCubeTextureView>,
  // uniforms:  mut HashMap<TypeId, Box<dyn Any>>,
}
