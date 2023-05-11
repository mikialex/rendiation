use crate::*;

/// The actual gpu data
///
/// we could customize the stream trait's context to avoid too much arc clone in update logic
#[pin_project::pin_project]
#[derive(Clone)]
pub struct ContentGPUSystem {
  pub(crate) gpu: ResourceGPUCtx,
  pub model_ctx: GPUModelResourceCtx,
  pub bindable_ctx: ShareBindableResourceCtx,
  pub models: Arc<RwLock<StreamMap<usize, ReactiveModelGPUType>>>,
  pub custom_storage: Arc<RefCell<AnyMap>>,
}

pub type ReactiveModelRenderComponentDeltaSource = impl Stream<Item = RenderComponentDeltaFlag>;

impl ContentGPUSystem {
  pub fn new(gpu: &GPU) -> Self {
    let bindable_ctx = ShareBindableResourceCtx::new(gpu);
    let gpu = ResourceGPUCtx::new(gpu, Default::default());

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
      custom_storage: Arc::new(RefCell::new(AnyMap::new())),
    }
  }

  pub fn get_or_create_reactive_model_render_component_delta_source(
    &self,
    model: &ModelType,
  ) -> Option<ReactiveModelRenderComponentDeltaSource> {
    self
      .models
      .write()
      .unwrap()
      .get_or_insert_with(model.guid()?, || {
        model.create_scene_reactive_gpu(&self.model_ctx).unwrap()
      })
      .create_render_component_delta_stream()
      .into()
  }
}

impl Stream for ContentGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    do_updates_by(&mut this.bindable_ctx, cx, |_| {});
    do_updates_by(&mut this.model_ctx, cx, |_| {});

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<usize, ReactiveModelGPUType> = &mut models;
    do_updates_by(models, cx, |_| {});
    Poll::Pending
  }
}

#[derive(Clone)]
#[pin_project::pin_project]
pub struct GPUModelResourceCtx {
  pub shared: ShareBindableResourceCtx,
  pub materials: Arc<RwLock<StreamMap<usize, MaterialGPUInstance>>>,
  pub meshes: Arc<RwLock<StreamMap<usize, MeshGPUInstance>>>,
}

impl Stream for GPUModelResourceCtx {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    do_updates_by(&mut this.shared, cx, |_| {});

    let mut materials = this.materials.write().unwrap();
    let materials: &mut StreamMap<usize, MaterialGPUInstance> = &mut materials;
    do_updates_by(materials, cx, |_| {});

    let mut meshes = this.meshes.write().unwrap();
    let meshes: &mut StreamMap<usize, MeshGPUInstance> = &mut meshes;
    do_updates_by(meshes, cx, |_| {});

    Poll::Pending
  }
}

#[derive(Clone)]
#[pin_project::pin_project]
pub struct ShareBindableResourceCtx {
  pub gpu: ResourceGPUCtx,
  pub custom_storage: Arc<RwLock<AnyMap>>,

  pub sampler: Arc<RwLock<StreamMap<usize, ReactiveGPUSamplerViewSource>>>,
  pub texture_2d: Arc<RwLock<StreamMap<usize, ReactiveGPU2DTextureViewSource>>>,
  pub texture_cube: Arc<RwLock<StreamMap<usize, ReactiveGPUCubeTextureViewSource>>>,
  // share uniform buffers
  // share vertex buffers
}

impl Stream for ShareBindableResourceCtx {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mut sampler = this.sampler.write().unwrap();
    let sampler: &mut StreamMap<usize, ReactiveGPUSamplerViewSource> = &mut sampler;
    do_updates_by(sampler, cx, |_| {});

    let mut texture_2d = this.texture_2d.write().unwrap();
    let texture_2d: &mut StreamMap<usize, ReactiveGPU2DTextureViewSource> = &mut texture_2d;
    do_updates_by(texture_2d, cx, |_| {});

    let mut texture_cube = this.texture_cube.write().unwrap();
    let texture_cube: &mut StreamMap<usize, ReactiveGPUCubeTextureViewSource> = &mut texture_cube;
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
