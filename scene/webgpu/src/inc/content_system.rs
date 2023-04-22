use crate::*;

/// The actual gpu data
///
/// we could customize the stream trait's context to avoid too much arc clone in update logic
#[pin_project::pin_project]
pub struct GlobalGPUSystem {
  gpu: ResourceGPUCtx,
  model_ctx: GPUModelResourceCtx,
  bindable_ctx: ShareBindableResourceCtx,
  texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureViewSource>>>,
  texture_cube: Arc<RwLock<StreamMap<ReactiveGPUCubeTextureViewSource>>>,
  // uniforms: HashMap<TypeId, Box<dyn Any>>,
  materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  // meshes: StreamMap<ReactiveRenderComponent>,
  #[pin]
  pub models: Arc<RwLock<StreamMap<ModelGPUReactive>>>,
}

impl GlobalGPUSystem {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    let gpu = ResourceGPUCtx::new(gpu, mipmap_gen);

    let bindable_ctx = ShareBindableResourceCtx {
      gpu: gpu.clone(),
      texture_2d: Default::default(),
      texture_cube: Default::default(),
    };

    let model_ctx = GPUModelResourceCtx {
      shared: bindable_ctx.clone(),
      materials: Default::default(),
    };

    let texture_2d = bindable_ctx.texture_2d.clone();
    let texture_cube = bindable_ctx.texture_cube.clone();
    let materials = model_ctx.materials.clone();

    Self {
      gpu,
      bindable_ctx,
      model_ctx,
      texture_2d,
      texture_cube,
      materials,
      models: Default::default(),
    }
  }
}

impl Stream for GlobalGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mut texture_2d = this.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<ReactiveGPU2DTextureViewSource> = &mut texture_2d;
    do_updates_by(texture_2d, cx, |_| {});

    let mut materials = this.materials.write().unwrap();
    let materials: &mut StreamMap<MaterialGPUInstance> = &mut materials;
    do_updates_by(materials, cx, |_| {});

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<ModelGPUReactive> = &mut models;
    do_updates_by(models, cx, |_| {});
    Poll::Pending
  }
}

#[derive(Clone)]
pub struct GPUModelResourceCtx {
  pub shared: ShareBindableResourceCtx,
  pub materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  // meshes: StreamMap<ReactiveRenderComponent>,
}

pub struct GPUResourceCache {
  pub scene: GPUResourceSceneCache,
  pub bindables: ShareBindableResourceCtx,
  pub custom_storage: AnyMap,
  pub cameras: CameraGPUMap,
  pub nodes: NodeGPUMap,
}

impl GPUResourceCache {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      scene: Default::default(),
      bindables: ShareBindableResourceCtx::new(gpu),
      custom_storage: AnyMap::new(),
      cameras: Default::default(),
      nodes: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    let mut texture_2d = self.bindables.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<ReactiveGPU2DTextureViewSource> = &mut texture_2d;
    do_updates(texture_2d, |_| {});
  }
}

#[derive(Default)]
pub struct GPULightCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}
#[derive(Default)]
pub struct GPUMaterialCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}
#[derive(Default)]
pub struct GPUMeshCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Default)]
pub struct GPUResourceSceneCache {
  pub materials: GPUMaterialCache,
  pub lights: GPULightCache,
  pub meshes: GPUMeshCache,
}

#[derive(Clone)]
pub struct ShareBindableResourceCtx {
  pub gpu: ResourceGPUCtx,
  pub texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureViewSource>>>,
  pub texture_cube: Arc<RwLock<StreamMap<ReactiveGPUCubeTextureViewSource>>>,
  // uniforms
}

impl ShareBindableResourceCtx {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      gpu: ResourceGPUCtx::new(gpu, Default::default()),
      texture_2d: Default::default(),
      texture_cube: Default::default(),
    }
  }
}
