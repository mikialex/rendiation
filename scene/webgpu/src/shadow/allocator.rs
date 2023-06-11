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
          view_formats: &[],
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
          mag_filter: webgpu::FilterMode::Linear,
          min_filter: webgpu::FilterMode::Linear,
          mipmap_filter: webgpu::FilterMode::Nearest,
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
          array_layer_count: Some(1),
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
