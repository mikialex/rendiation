use crate::*;

/// The actual gpu data
///
/// we could customize the stream trait's context to avoid too much arc clone in update logic
#[pin_project::pin_project]
pub struct GlobalGPUSystem {
  gpu: ResourceGPUCtx,
  model_ctx: GPUModelResourceCtx,
  bindable_ctx: ShareBindableResourceCtx,
  pub models: Arc<RwLock<StreamMap<ModelGPUReactive>>>,
}

impl GlobalGPUSystem {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    let bindable_ctx = ShareBindableResourceCtx::new(gpu);
    let gpu = ResourceGPUCtx::new(gpu, mipmap_gen);

    let model_ctx = GPUModelResourceCtx {
      shared: bindable_ctx.clone(),
      materials: Default::default(),
      meshes: Default::default(),
    };

    Self {
      gpu,
      bindable_ctx,
      model_ctx,
      models: Default::default(),
    }
  }
}

impl Stream for GlobalGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    do_updates_by(&mut this.bindable_ctx, cx, |_| {});
    do_updates_by(&mut this.model_ctx, cx, |_| {});

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<ModelGPUReactive> = &mut models;
    do_updates_by(models, cx, |_| {});
    Poll::Pending
  }
}

#[derive(Clone)]
#[pin_project::pin_project]
pub struct GPUModelResourceCtx {
  pub shared: ShareBindableResourceCtx,
  pub materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  pub meshes: Arc<RwLock<StreamMap<MeshGPUInstance>>>,
}

impl Stream for GPUModelResourceCtx {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    do_updates_by(&mut this.shared, cx, |_| {});

    let mut materials = this.materials.write().unwrap();
    let materials: &mut StreamMap<MaterialGPUInstance> = &mut materials;
    do_updates_by(materials, cx, |_| {});

    let mut meshes = this.meshes.write().unwrap();
    let meshes: &mut StreamMap<MeshGPUInstance> = &mut meshes;
    do_updates_by(meshes, cx, |_| {});

    Poll::Pending
  }
}

pub struct GPUResourceCache {
  pub scene: GPUResourceSceneCache,
  pub bindables: ShareBindableResourceCtx,
  pub materials: Arc<RwLock<StreamMap<MaterialGPUInstance>>>,
  pub custom_storage: AnyMap,
  pub cameras: CameraGPUMap,
  pub nodes: NodeGPUMap,
}

impl GPUResourceCache {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      scene: Default::default(),
      bindables: ShareBindableResourceCtx::new(gpu),
      materials: Default::default(),
      custom_storage: AnyMap::new(),
      cameras: Default::default(),
      nodes: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    let mut texture_2d = self.bindables.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<ReactiveGPU2DTextureViewSource> = &mut texture_2d;
    do_updates(texture_2d, |_| {});

    let mut materials = self.materials.write().unwrap();
    let materials: &mut StreamMap<MaterialGPUInstance> = &mut materials;
    do_updates(materials, |_| {});
  }
}

#[derive(Default)]
pub struct GPULightCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Default)]
pub struct GPUResourceSceneCache {
  pub lights: GPULightCache,
}

#[derive(Clone)]
#[pin_project::pin_project]
pub struct ShareBindableResourceCtx {
  pub gpu: ResourceGPUCtx,
  pub custom_storage: Arc<RwLock<AnyMap>>,

  pub sampler: Arc<RwLock<StreamMap<ReactiveGPUSamplerViewSource>>>,
  pub texture_2d: Arc<RwLock<StreamMap<ReactiveGPU2DTextureViewSource>>>,
  pub texture_cube: Arc<RwLock<StreamMap<ReactiveGPUCubeTextureViewSource>>>,
  // share uniform buffers
  // share vertex buffers
}

impl Stream for ShareBindableResourceCtx {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mut sampler = this.sampler.write().unwrap();
    let sampler: &mut StreamMap<ReactiveGPUSamplerViewSource> = &mut sampler;
    do_updates_by(sampler, cx, |_| {});

    let mut texture_2d = this.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<ReactiveGPU2DTextureViewSource> = &mut texture_2d;
    do_updates_by(texture_2d, cx, |_| {});

    let mut texture_cube = this.texture_cube.write().unwrap();
    let texture_cube: &mut StreamMap<ReactiveGPUCubeTextureViewSource> = &mut texture_cube;
    do_updates_by(texture_cube, cx, |_| {});

    Poll::Pending
  }
}

impl ShareBindableResourceCtx {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      custom_storage: Arc::new(RwLock::new(AnyMap::new())),
      gpu: ResourceGPUCtx::new(gpu, Default::default()),
      sampler: Default::default(),
      texture_2d: Default::default(),
      texture_cube: Default::default(),
    }
  }
}
