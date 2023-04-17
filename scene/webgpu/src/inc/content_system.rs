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
  models: StreamMap<ReactiveRenderComponent>,
}

impl GlobalGPUSystem {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    let gpu = ResourceGPUCtx::new(gpu, mipmap_gen);
    Self {
      gpu,
      texture_2d: Default::default(),
      materials: Default::default(),
    }
  }
}

impl Stream for GlobalGPUSystem {
  type Item = ();

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    todo!()
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
