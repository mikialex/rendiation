use __core::num::NonZeroU32;

use crate::*;

/// In shader, we want a single texture binding for all shadowmap with same format.
/// All shadowmap are allocated in one texture with multi layers.
#[derive(Default)]
pub struct ShadowMapAllocator {
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl ShaderPassBuilder for ShadowMapAllocator {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let inner = self.inner.borrow();
    let inner = inner.result.as_ref().unwrap();
    ctx.binding.bind(&inner.map, SB::Pass);
    ctx.binding.bind(&inner.sampler, SB::Pass);
  }
}

impl ShaderGraphProvider for ShadowMapAllocator {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let inner = self.inner.borrow();
    let inner = &inner.result.as_ref().unwrap();
    builder.fragment(|builder, binding| {
      let map = binding.uniform_by(&inner.map, SB::Pass);
      let sampler = binding.uniform_by(&inner.sampler, SB::Pass);
      builder.register::<BasicShadowMap>(map);
      builder.register::<BasicShadowMapSampler>(sampler);
      Ok(())
    })
  }
}

#[derive(Default)]
pub struct ShadowMapAllocatorImpl {
  id: usize,
  result: Option<ShadowMapAllocationInfo>,
  requirements: LinkedHashMap<usize, Size>,
}

impl ShadowMapAllocatorImpl {
  fn check_rebuild(&mut self, gpu: &GPU) -> &ShadowMapAllocationInfo {
    self.result.get_or_insert_with(|| {
      // we only impl naive strategy now, just ignore the size requirement

      let size = self.requirements.len();
      let map = GPUTexture::create(
        webgpu::TextureDescriptor {
          label: "shadow-maps".into(),
          size: webgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: size as u32,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: webgpu::TextureDimension::D2,
          format: webgpu::TextureFormat::Depth32Float,
          usage: webgpu::TextureUsages::TEXTURE_BINDING | webgpu::TextureUsages::RENDER_ATTACHMENT,
        },
        &gpu.device,
      );
      let map = map.create_view(Default::default()).try_into().unwrap();

      let mapping = self
        .requirements
        .iter()
        .enumerate()
        .map(|(i, (v, _))| {
          (
            *v,
            ShadowMapAddressInfo {
              layer_index: i as i32,
              size: Vec2::zero(),
              offset: Vec2::zero(),
              ..Zeroable::zeroed()
            },
          )
        })
        .collect();

      let sampler = GPUComparisonSampler::create(
        webgpu::SamplerDescriptor {
          compare: webgpu::CompareFunction::Greater.into(),
          ..Default::default()
        },
        &gpu.device,
      )
      .create_view(());

      ShadowMapAllocationInfo {
        map,
        mapping,
        sampler,
      }
    })
  }
}

struct ShadowMapAllocationInfo {
  map: GPU2DArrayDepthTextureView,
  sampler: GPUComparisonSamplerView,
  mapping: LinkedHashMap<usize, ShadowMapAddressInfo>,
}

#[derive(Clone)]
pub struct ShadowMap {
  inner: Rc<ShadowMapInner>,
}

struct ShadowMapInner {
  id: usize,
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl Drop for ShadowMapInner {
  fn drop(&mut self) {
    let mut inner = self.inner.borrow_mut();
    inner.requirements.remove(&self.id);
    if let Some(result) = &mut inner.result {
      result.mapping.remove(&self.id);
    }
  }
}

impl ShadowMap {
  pub fn get_write_view(&self, gpu: &GPU) -> (GPU2DTextureView, ShadowMapAddressInfo) {
    let mut inner = self.inner.inner.borrow_mut();
    let id = self.inner.id;
    let result = inner.check_rebuild(gpu);
    let base_array_layer = result.mapping.get(&id).unwrap().layer_index as u32;

    (
      result
        .map
        .resource
        .create_view(webgpu::TextureViewDescriptor {
          label: Some("shadow-write-view"),
          dimension: Some(webgpu::TextureViewDimension::D2),
          base_array_layer,
          array_layer_count: NonZeroU32::new(1).unwrap().into(),
          ..Default::default()
        })
        .try_into()
        .unwrap(),
      *result.mapping.get(&id).unwrap(),
    )
  }
}

impl ShadowMapAllocator {
  pub fn allocate(&self, resolution: Size) -> ShadowMap {
    let mut inner = self.inner.borrow_mut();
    inner.id += 1;

    let id = inner.id;
    inner.requirements.insert(id, resolution);

    let s_inner = ShadowMapInner {
      id,
      inner: self.inner.clone(),
    };
    ShadowMap {
      inner: Rc::new(s_inner),
    }
  }
}

pub trait ShadowCollection: RenderComponentAny + RebuildAbleGPUCollectionBase {
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: RenderComponentAny + RebuildAbleGPUCollectionBase + Any> ShadowCollection for T {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

pub struct ShadowMapSystem {
  pub shadow_collections: LinkedHashMap<TypeId, Box<dyn ShadowCollection>>,
  pub maps: ShadowMapAllocator,
  pub sampler: RawComparisonSampler,
}

pub const SHADOW_MAX: usize = 8;
pub type ShadowList<T> = ClampedUniformList<T, SHADOW_MAX>;

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

impl Default for BasicShadowMapInfoList {
  fn default() -> Self {
    Self {
      list: ShadowList::default_with(SB::Pass),
    }
  }
}

impl ShaderGraphProvider for BasicShadowMapInfoList {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let list = binding.uniform_by(self.list.gpu.as_ref().unwrap(), SB::Pass);
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

impl ShadowMapSystem {
  pub fn new(gpu: &GPU) -> Self {
    let mut sampler = SamplerDescriptor::default();
    sampler.compare = CompareFunction::Less.into();
    Self {
      shadow_collections: Default::default(),
      maps: Default::default(),
      sampler: gpu.device.create_and_cache_com_sampler(sampler),
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
