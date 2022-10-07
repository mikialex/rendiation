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
    todo!()
  }
}

#[derive(Default)]
pub struct ShadowMapAllocatorImpl {
  id: usize,
  result: Option<ShadowMapAllocationInfo>,
  requirements: HashMap<usize, Size>,
}

impl ShadowMapAllocatorImpl {
  fn check_rebuild(&mut self, gpu: &GPU) -> &GPU2DArrayTextureView {
    &self
      .result
      .get_or_insert_with(|| {
        // we only impl naive strategy now, just ignore the size requirement

        let size = self.requirements.len();
        let map = GPUTexture::create(
          webgpu::TextureDescriptor {
            label: None,
            size: webgpu::Extent3d {
              width: 512,
              height: 512,
              depth_or_array_layers: size as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: webgpu::TextureDimension::D2,
            format: webgpu::TextureFormat::Depth32Float,
            usage: webgpu::TextureUsages::TEXTURE_BINDING
              | webgpu::TextureUsages::COPY_DST
              | webgpu::TextureUsages::RENDER_ATTACHMENT,
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
                layer_index: i as u32,
                size: Vec2::zero(),
                offset: Vec2::zero(),
                ..Zeroable::zeroed()
              },
            )
          })
          .collect();

        ShadowMapAllocationInfo { map, mapping }
      })
      .map
  }
}

struct ShadowMapAllocationInfo {
  map: GPU2DArrayTextureView,
  mapping: HashMap<usize, ShadowMapAddressInfo>,
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
  pub fn get_write_view(&self, gpu: &GPU) -> GPU2DTextureView {
    let inner = self.inner.inner.borrow();
    let result = inner.result.as_ref().unwrap();
    let base_array_layer = result.mapping.get(&inner.id).unwrap().layer_index;

    result
      .map
      .resource
      .create_view(webgpu::TextureViewDescriptor {
        base_array_layer,
        array_layer_count: NonZeroU32::new(1).unwrap().into(),
        ..Default::default()
      })
      .try_into()
      .unwrap()
  }

  pub fn get_address_info(&self, gpu: &GPU) -> ShadowMapAddressInfo {
    let inner = self.inner.inner.borrow();
    let result = inner.result.as_ref().unwrap();
    *result.mapping.get(&inner.id).unwrap()
  }
}

impl ShadowMapAllocator {
  pub fn allocate(&self, gpu: &GPU, resolution: Size) -> ShadowMap {
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
