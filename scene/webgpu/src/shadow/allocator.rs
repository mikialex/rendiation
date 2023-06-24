use crate::*;

/// In shader, we want a single texture binding for all shadowmap with same format.
/// All shadowmap are allocated in one texture with multi layers.
#[derive(Default)]
pub struct ShadowMapAllocator {
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl ShadowMapAllocator {
  pub fn allocate(&self, size_requirement: Size) -> ShadowMap {
    let mut inner = self.inner.borrow_mut();
    inner.id += 1;

    let id = inner.id;

    let (sender, receiver) = futures::channel::mpsc::unbounded();

    if inner.size_all.depth_or_array_layers == self.allocation.len() {
      // resize and emit changes
      let map = GPUTexture::create(
        webgpu::TextureDescriptor {
          label: "shadow-maps".into(),
          size: inner.size * 2,
          mip_level_count: 1,
          sample_count: 1,
          dimension: webgpu::TextureDimension::D2,
          format: webgpu::TextureFormat::Depth32Float,
          view_formats: &[],
          usage: webgpu::TextureUsages::TEXTURE_BINDING | webgpu::TextureUsages::RENDER_ATTACHMENT,
        },
        &inner.device,
      );
      inner.map = map.create_view(Default::default()).try_into().unwrap();
      inner
        .allocations
        .values_mut()
        .enumerate()
        .for_each(|(layer, alloc)| {
          alloc.info = ShadowMapAddressInfo {
            layer_index: layer,
            size: todo!(),
            offset: Vec2::Zero(),
            ..Zeroable::zeroed()
          };
          alloc.sender.unbounded_send(alloc.info);
        })
    }
    let current = ShadowMapAddressInfo {
      layer_index: self.allocation.len(),
      size: todo!(),
      offset: Vec2::Zero(),
      ..Zeroable::zeroed()
    };

    let allocation = LiveAllocation {
      size_requirement,
      info: current,
      sender,
    };

    inner.allocations.insert(id, allocation);

    ShadowMap {
      id,
      size: size_requirement,
      inner: self.inner.clone(),
      current,
      deltas: Box::new(receiver),
    }
  }
}

only_fragment!(BasicShadowMap, ShaderDepthTexture2DArray);
only_fragment!(BasicShadowMapSampler, ShaderCompareSampler);

impl ShaderPassBuilder for ShadowMapAllocator {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let inner = self.inner.borrow();
    let inner = inner.result.as_ref().unwrap();
    ctx.binding.bind(&inner.map);
    ctx.binding.bind(&inner.sampler);
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
      let map = binding.uniform_by(&inner.map);
      let sampler = binding.uniform_by(&inner.sampler);
      builder.register::<BasicShadowMap>(map);
      builder.register::<BasicShadowMapSampler>(sampler);
      Ok(())
    })
  }
}

pub struct ShadowMapAllocatorImpl {
  id: usize,
  device: GPUDevice,
  map: GPU2DArrayDepthTextureView,
  sampler: GPUComparisonSamplerView,
  size_all: webgpu::Extent3d,
  allocations: HashMap<usize, LiveAllocation>,
}

struct LiveAllocation {
  size_requirement: Size,
  info: ShadowMapAddressInfo,
  sender: futures::channel::mpsc::UnboundedSender<ShadowMapAddressInfo>,
}

impl ShadowMapAllocatorImpl {
  fn new(device: &GPUDevice) -> Self {
    let init_size = webgpu::Extent3d {
      width: 512,
      height: 512,
      depth_or_array_layers: 5 as u32,
    };

    let map = GPUTexture::create(
      webgpu::TextureDescriptor {
        label: "shadow-maps".into(),
        size: init_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: webgpu::TextureDimension::D2,
        format: webgpu::TextureFormat::Depth32Float,
        view_formats: &[],
        usage: webgpu::TextureUsages::TEXTURE_BINDING | webgpu::TextureUsages::RENDER_ATTACHMENT,
      },
      device,
    );
    let map = map.create_view(Default::default()).try_into().unwrap();

    let mapping = Default::default();

    let sampler = GPUComparisonSampler::create(
      webgpu::SamplerDescriptor {
        mag_filter: webgpu::FilterMode::Linear,
        min_filter: webgpu::FilterMode::Linear,
        mipmap_filter: webgpu::FilterMode::Nearest,
        compare: webgpu::CompareFunction::Greater.into(),
        ..Default::default()
      },
      device,
    )
    .create_view(());

    Self {
      id: 0,
      map,
      device: device.clone(),
      size_all: init_size,
      allocations: mapping,
      sampler,
    }
  }
}

#[pin_project::pin_project]
pub struct ShadowMap {
  id: usize,
  size: Size,
  current: ShadowMapAddressInfo,
  #[pin]
  deltas: Box<dyn Stream<Item = ShadowMapAddressInfo>>,
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl Drop for ShadowMap {
  fn drop(&mut self) {
    let mut inner = self.inner.borrow_mut();
    inner.allocations.remove(&self.id);
  }
}

impl Stream for ShadowMap {
  type Item = ShadowMapAddressInfo;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(r) = this.deltas.poll_next(cx) {
      if let Some(r) = r {
        self.current = r;
      }
      return Poll::Ready(r);
    } else {
      return Poll::Pending;
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
