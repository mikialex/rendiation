use crate::*;

mod allocator;
pub use allocator::*;

mod basic;
pub use basic::*;

mod sampling;
pub use sampling::*;

pub struct ShadowMapSystem {
  pub single_proj_sys: Arc<RwLock<SingleProjectShadowMapSystem>>,
  pub maps: ShadowMapAllocator,
  pub sampler: RawSampler,
}

impl ShadowMapSystem {
  pub fn new(gpu: ResourceGPUCtx, derives: SceneNodeDeriveSystem) -> Self {
    let maps = ShadowMapAllocator::new(gpu.clone());
    let sampler = SamplerDescriptor {
      compare: CompareFunction::Less.into(),
      ..Default::default()
    };
    let single_proj_sys = SingleProjectShadowMapSystem::new(gpu.clone(), maps.clone(), derives);
    Self {
      single_proj_sys: Arc::new(RwLock::new(single_proj_sys)),
      sampler: gpu.device.create_and_cache_sampler(sampler),
      maps,
    }
  }
  pub fn maintain(&mut self, gpu_cameras: &mut SceneCameraGPUSystem, cx: &mut Context) {
    self
      .single_proj_sys
      .write()
      .unwrap()
      .poll_updates(gpu_cameras, cx)
  }
}

impl ShaderPassBuilder for ShadowMapSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.single_proj_sys.read().unwrap().list.setup_pass(ctx);
    self.maps.setup_pass(ctx)
  }
}

impl ShaderHashProvider for ShadowMapSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .single_proj_sys
      .read()
      .unwrap()
      .list
      .hash_pipeline(hasher);
    // self.maps.hash_pipeline(ctx) // we don't need this now?
  }
}

impl GraphicsShaderProvider for ShadowMapSystem {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.single_proj_sys.read().unwrap().list.build(builder)?;
    self.maps.build(builder)
  }
}

pub const SHADOW_MAX: usize = 8;
pub type ShadowList<T> = ClampedUniformList<T, SHADOW_MAX>;

#[derive(Default)]
pub struct BasicShadowMapInfoList {
  pub list: ShadowList<BasicShadowMapInfo>,
}

impl GraphicsShaderProvider for BasicShadowMapInfoList {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let list = binding.bind_by(self.list.gpu.as_ref().unwrap());
      builder.register::<BasicShadowMapInfoGroup>(list);
      Ok(())
    })
  }
}
impl ShaderHashProvider for BasicShadowMapInfoList {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.list.hash_pipeline(hasher)
  }
}
impl ShaderPassBuilder for BasicShadowMapInfoList {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.list.setup_pass(ctx)
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct BasicShadowMapInfo {
  pub shadow_camera: CameraGPUTransform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct ShadowBias {
  pub bias: f32,
  pub normal_bias: f32,
}

impl ShadowBias {
  pub fn new(bias: f32, normal_bias: f32) -> Self {
    Self {
      bias,
      normal_bias,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct ShadowMapAddressInfo {
  pub layer_index: i32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LightShadowAddressInfo {
  pub index: u32,
  pub enabled: u32,
}

impl LightShadowAddressInfo {
  pub fn new(enabled: bool, index: u32) -> Self {
    Self {
      enabled: enabled.into(),
      index,
      ..Zeroable::zeroed()
    }
  }
}

pub fn compute_shadow_position(
  builder: &ShaderFragmentBuilderView,
  shadow_info: ENode<BasicShadowMapInfo>,
) -> Result<Node<Vec3<f32>>, ShaderBuildError> {
  // another way to compute this is in vertex shader, maybe we will try it later.
  let bias = shadow_info.bias.expand();
  let world_position = builder.query::<FragmentWorldPosition>()?;
  let world_normal = builder.query::<FragmentWorldNormal>()?;

  // apply normal bias
  let world_position = world_position + bias.normal_bias * world_normal;

  let shadow_position =
    shadow_info.shadow_camera.expand().view_projection * (world_position, val(1.)).into();

  let shadow_position = shadow_position.xyz() / shadow_position.w().splat();

  // convert to uv space and apply offset bias
  Ok(
    shadow_position * val(Vec3::new(0.5, -0.5, 1.))
      + val(Vec3::new(0.5, 0.5, 0.))
      + (val(0.), val(0.), bias.bias).into(),
  )
}
