use rendiation_mesh_gpu_system::GPUBindlessMeshSystem;
use rendiation_texture::GPUBufferImage;

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
  pub models: Arc<RwLock<StreamMap<u64, ReactiveModelGPUType>>>,
  pub custom_storage: Arc<RefCell<AnyMap>>,
}

pub type ReactiveModelRenderComponentDeltaSource = impl Stream<Item = RenderComponentDeltaFlag>;

impl ContentGPUSystem {
  pub fn new(gpu: &GPU, config: BindableResourceConfig) -> Self {
    let bindable_ctx = ShareBindableResourceCtx::new(gpu, config);
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
    model: &ModelEnum,
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

impl FusedStream for ContentGPUSystem {
  fn is_terminated(&self) -> bool {
    false
  }
}
impl Stream for ContentGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.bindable_ctx.poll_until_pending_not_care_result(cx);
    this.model_ctx.poll_until_pending_not_care_result(cx);

    this
      .models
      .write()
      .unwrap()
      .poll_until_pending_not_care_result(cx);
    Poll::Pending
  }
}

#[derive(Clone)]
#[pin_project::pin_project]
pub struct GPUModelResourceCtx {
  pub shared: ShareBindableResourceCtx,
  pub materials: Arc<RwLock<StreamMap<u64, MaterialGPUInstance>>>,
  pub meshes: Arc<RwLock<StreamMap<u64, MeshGPUInstance>>>,
}

impl Stream for GPUModelResourceCtx {
  type Item = ();

  #[rustfmt::skip]
  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();

    this.shared.poll_until_pending_not_care_result(cx);

    this.materials.write().unwrap().poll_until_pending_not_care_result(cx);
    this.meshes.write().unwrap().poll_until_pending_not_care_result(cx);

    Poll::Pending
  }
}
impl FusedStream for GPUModelResourceCtx {
  fn is_terminated(&self) -> bool {
    false
  }
}

#[derive(Clone)]
#[pin_project::pin_project]
pub struct ShareBindableResourceCtx {
  pub gpu: ResourceGPUCtx,
  pub custom_storage: Arc<RwLock<AnyMap>>,

  pub bindless_mesh: Option<GPUBindlessMeshSystem>,

  pub binding_sys: GPUTextureBindingSystem,
  pub default_sampler: IncrementalSignalPtr<TextureSampler>,
  pub default_texture_2d: SceneTexture2D,
  pub sampler: Arc<RwLock<StreamMap<u64, ReactiveGPUSamplerViewSource>>>,
  pub texture_2d: Arc<RwLock<StreamMap<u64, ReactiveGPU2DTextureViewSource>>>,
  pub texture_cube: Arc<RwLock<StreamMap<u64, ReactiveGPUCubeTextureViewSource>>>,
  // share uniform buffers
  // share storage buffers
  // share vertex buffers
}

impl Stream for ShareBindableResourceCtx {
  type Item = ();

  #[rustfmt::skip]
  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.sampler.write().unwrap().poll_until_pending_not_care_result(cx);
    this.texture_2d.write().unwrap().poll_until_pending_not_care_result(cx);
    this.texture_cube.write().unwrap().poll_until_pending_not_care_result(cx);

    this.binding_sys.poll_until_pending_not_care_result(cx);

    if let Some(bindless_mesh) = this.bindless_mesh {
      bindless_mesh.maintain();
    }

    Poll::Pending
  }
}
impl FusedStream for ShareBindableResourceCtx {
  fn is_terminated(&self) -> bool {
    false
  }
}

#[derive(Clone, Copy, Debug)]
pub struct BindableResourceConfig {
  /// decide if should enable texture bindless support if platform hardware supported
  pub prefer_bindless_texture: bool,
  /// decide if should enable mesh bindless (multi indirect draw) support if platform hardware
  /// supported
  pub prefer_bindless_mesh: bool,
}

impl ShareBindableResourceCtx {
  pub fn new(gpu: &GPU, config: BindableResourceConfig) -> Self {
    // create a 1x1 white pixel as the default texture;
    let default_texture_2d = GPUBufferImage {
      data: vec![255, 255, 255, 255],
      format: TextureFormat::Rgba8UnormSrgb,
      size: Size::from_u32_pair_min_one((1, 1)),
    };
    let default_texture_2d = SceneTexture2DType::GPUBufferImage(default_texture_2d).into_ptr();
    let sys = Self {
      bindless_mesh: config
        .prefer_bindless_mesh
        .then(|| GPUBindlessMeshSystem::new(gpu))
        .flatten(),
      binding_sys: GPUTextureBindingSystem::new(gpu, config.prefer_bindless_texture, 8192),
      default_texture_2d,
      default_sampler: Default::default(),
      custom_storage: Arc::new(RwLock::new(AnyMap::new())),
      gpu: ResourceGPUCtx::new(gpu, Default::default()),
      sampler: Default::default(),
      texture_2d: Default::default(),
      texture_cube: Default::default(),
    };

    // make sure the binding sys has correct default value as the first element inserted
    // this is essential, because under wgpu, even if we enabled partial bind, we require have at
    // least one element in bind array, and we also rely on check the handle equals zero to decide
    // if the item actually exist in shader
    let _ = sys.get_or_create_reactive_gpu_sampler(&sys.default_sampler);
    let _ = sys.get_or_create_reactive_gpu_texture2d(&sys.default_texture_2d);

    sys
  }
}
