use rendiation_mesh_gpu_system::GPUBindlessMeshSystem;
use rendiation_texture::GPUBufferImage;

use crate::*;

pub struct ContentGPUSystem {
  pub(crate) gpu: ResourceGPUCtx,
  pub model_ctx: GPUModelResourceCtx,
  pub bindable_ctx: ShareBindableResourceCtx,
  pub models: Arc<RwLock<StreamMap<u64, ReactiveModelGPUType>>>,
}

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

pub struct ShareBindableResourceCtx {
  pub bindless_mesh: Option<GPUBindlessMeshSystem>,

  pub binding_sys: GPUTextureBindingSystem,
  pub default_sampler: IncrementalSignalPtr<TextureSampler>,
  pub default_texture_2d: SceneTexture2D,
  pub sampler: Box<dyn ReactiveCollection<AllocIdx<TextureSampler>, GPUSamplerView>>,
  pub texture_2d: Box<dyn ReactiveCollection<AllocIdx<SceneTexture2DType>, GPU2DTextureView>>,
  pub texture_cube: Box<dyn ReactiveCollection<AllocIdx<SceneTextureCubeImpl>, GPUCubeTextureView>>,
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
      binding_sys: GPUTextureBindingSystem::new(gpu, config.prefer_bindless_texture),
      default_texture_2d,
      default_sampler: Default::default(),
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
