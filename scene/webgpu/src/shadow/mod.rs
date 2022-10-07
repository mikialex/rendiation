use crate::*;

/// In shader, we want a single texture binding for all shadowmap with same format.
/// All shadowmap are allocated in one texture with multi layers.
#[derive(Default)]
pub struct ShadowMapAllocator {
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl ShaderPassBuilder for ShadowMapAllocator {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    todo!()
  }
}

impl ShadowMapAllocator {
  pub fn shadow_given_light(light_id: Node<u32>, world_position: Node<Vec3<f32>>) -> Node<f32> {
    todo!()
  }
}

#[derive(Default)]
pub struct ShadowMapAllocatorImpl {
  id: usize,
  gpu: Option<GPU2DTexture>,
  mapping: HashMap<usize, ShadowMapAllocationInfo>,
}

impl ShadowMapAllocatorImpl {
  fn check_rebuild(&mut self, gpu: &GPU) -> &GPU2DTexture {
    self.gpu.get_or_insert_with(|| todo!())
  }
}

struct ShadowMapAllocationInfo {
  require_size: Size,
  result: Option<ShadowMapAddressInfo>,
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
    self.inner.borrow_mut().mapping.remove(&self.id);
  }
}

impl ShadowMap {
  pub fn get_write_view(&self, gpu: &GPU) -> GPU2DTextureView {
    todo!()
  }

  pub fn get_address_info(&self, gpu: &GPU) -> ShadowMapAddressInfo {
    todo!()
  }
}

impl ShadowMapAllocator {
  pub fn allocate(&self, gpu: &GPU, resolution: Size) -> ShadowMap {
    todo!()
  }
}

pub trait ShadowCollection: Any + ShaderPassBuilder {
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: Any + ShaderPassBuilder> ShadowCollection for T {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

pub struct ShadowMapSystem {
  pub shadow_collections: LinkedHashMap<TypeId, Box<dyn ShadowCollection>>,
  pub maps: ShadowMapAllocator,
  pub sampler: RawComparisonSampler,
}

const SHADOW_MAX: usize = 8;
pub type ShadowList<T> = ClampedUniformList<T, SHADOW_MAX>;

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

  pub fn get_or_create_list<T: Std140>(&mut self) -> &mut ShadowList<T> {
    let lights = self
      .shadow_collections
      .entry(TypeId::of::<T>())
      .or_insert_with(|| Box::new(ShadowList::<T>::default_with(SB::Pass)));
    lights.as_any_mut().downcast_mut::<ShadowList<T>>().unwrap()
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

impl ShaderGraphProvider for ShadowMapSystem {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
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

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct ShadowMapAddressInfo {
  pub layer_index: u32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}
