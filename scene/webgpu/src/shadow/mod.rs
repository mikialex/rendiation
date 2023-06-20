use crate::*;

pub mod allocator;
pub use allocator::*;

pub mod basic;
pub use basic::*;

pub mod sampling;
pub use sampling::*;

pub struct ShadowMapSystem {
  pub shadow_collections: LinkedHashMap<TypeId, Box<dyn ShadowCollection>>,
  pub maps: ShadowMapAllocator,
  pub sampler: RawSampler,
}

pub trait ShadowCollection: RenderComponentAny + RebuildAbleGPUCollectionBase {
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: RenderComponentAny + RebuildAbleGPUCollectionBase + Any> ShadowCollection for T {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

impl ShadowMapSystem {
  pub fn new(gpu: &GPU) -> Self {
    let mut sampler = SamplerDescriptor::default();
    sampler.compare = CompareFunction::Less.into();
    Self {
      shadow_collections: Default::default(),
      maps: Default::default(),
      sampler: gpu.device.create_and_cache_sampler(sampler),
    }
  }

  pub fn before_update_scene(&mut self, _gpu: &GPU) {
    self
      .shadow_collections
      .iter_mut()
      .for_each(|(_, c)| c.reset());
  }

  pub fn after_update_scene(&mut self, gpu: &GPU) {
    self.shadow_collections.iter_mut().for_each(|(_, c)| {
      c.update_gpu(gpu);
    });
  }
}

impl ShaderPassBuilder for ShadowMapSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    for impls in self.shadow_collections.values() {
      impls.setup_pass(ctx)
    }
    self.maps.setup_pass(ctx)
  }
}

impl ShaderHashProvider for ShadowMapSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    for impls in self.shadow_collections.values() {
      impls.hash_pipeline(hasher)
    }
    // self.maps.hash_pipeline(ctx) // we don't need this now
  }
}

impl ShaderGraphProvider for ShadowMapSystem {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for impls in self.shadow_collections.values() {
      impls.build(builder)?;
    }
    self.maps.build(builder)
  }
}

pub const SHADOW_MAX: usize = 8;
pub type ShadowList<T> = ClampedUniformList<T, SHADOW_MAX>;

#[derive(Default)]
pub struct BasicShadowMapInfoList {
  pub list: ShadowList<BasicShadowMapInfo>,
}

impl RebuildAbleGPUCollectionBase for BasicShadowMapInfoList {
  fn reset(&mut self) {
    self.list.reset();
  }

  fn update_gpu(&mut self, gpu: &GPU) -> usize {
    self.list.update_gpu(gpu)
  }
}

impl ShaderGraphProvider for BasicShadowMapInfoList {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let list = binding.uniform_by(self.list.gpu.as_ref().unwrap());
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
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct BasicShadowMapInfo {
  pub shadow_camera: CameraGPUTransform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
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
