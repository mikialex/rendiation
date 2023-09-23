use crate::*;

/// In shader, we want a single texture binding for all shadowmap with same format.
/// All shadowmap are allocated in one texture with multi layers.
#[derive(Clone)]
pub struct ShadowMapAllocator {
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl ShadowMapAllocator {
  pub fn new(gpu: ResourceGPUCtx) -> Self {
    Self {
      inner: Rc::new(RefCell::new(ShadowMapAllocatorImpl::new(gpu))),
    }
  }

  pub fn allocate(&self, size_requirement: Size) -> ShadowMap {
    let (width, height) = size_requirement.into_usize();
    let mut inner = self.inner.borrow_mut();
    inner.id += 1;

    let id = inner.id;

    let (sender, receiver) = futures::channel::mpsc::unbounded();

    if inner.size_all.depth_or_array_layers == inner.allocations.len() as u32 {
      // resize and emit changes
      let map = GPUTexture::create(
        TextureDescriptor {
          label: "shadow-maps".into(),
          size: Extent3d {
            width: inner.size_all.width,
            height: inner.size_all.height,
            depth_or_array_layers: inner.size_all.depth_or_array_layers * 2,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: TextureDimension::D2,
          format: TextureFormat::Depth32Float,
          view_formats: &[],
          usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        },
        &inner.gpu.device,
      );
      inner.map = map.create_view(Default::default()).try_into().unwrap();
      inner
        .allocations
        .values_mut()
        .enumerate()
        .for_each(|(layer, alloc)| {
          alloc.info = ShadowMapAddressInfo {
            layer_index: layer as i32,
            size: Vec2::new(width as f32, height as f32),
            offset: Vec2::zero(),
            ..Zeroable::zeroed()
          };
          alloc.sender.unbounded_send(alloc.info).ok();
        })
    }
    let current = ShadowMapAddressInfo {
      layer_index: inner.allocations.len() as i32,
      size: Vec2::new(width as f32, height as f32),
      offset: Vec2::zero(),
      ..Zeroable::zeroed()
    };

    sender.unbounded_send(current).ok();
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

only_fragment!(BasicShadowMap, ShaderHandlePtr<ShaderDepthTexture2DArray>);
only_fragment!(BasicShadowMapSampler, ShaderHandlePtr<ShaderCompareSampler>);

impl ShaderPassBuilder for ShadowMapAllocator {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let inner = self.inner.borrow();
    ctx.binding.bind(&inner.map);
    ctx.binding.bind(&inner.sampler);
  }
}

impl GraphicsShaderProvider for ShadowMapAllocator {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let inner = self.inner.borrow();
    builder.fragment(|builder, binding| {
      let map = binding.bind_by(&inner.map);
      let sampler = binding.bind_by(&inner.sampler);
      builder.register::<BasicShadowMap>(map);
      builder.register::<BasicShadowMapSampler>(sampler);
      Ok(())
    })
  }
}

pub struct ShadowMapAllocatorImpl {
  id: usize,
  gpu: ResourceGPUCtx,
  map: GPU2DArrayDepthTextureView,
  sampler: GPUComparisonSamplerView,
  size_all: Extent3d,
  allocations: FastHashMap<usize, LiveAllocation>,
}

struct LiveAllocation {
  #[allow(dead_code)]
  size_requirement: Size,
  info: ShadowMapAddressInfo,
  sender: futures::channel::mpsc::UnboundedSender<ShadowMapAddressInfo>,
}

impl ShadowMapAllocatorImpl {
  fn new(gpu: ResourceGPUCtx) -> Self {
    let init_size = Extent3d {
      width: 512,
      height: 512,
      depth_or_array_layers: 5_u32,
    };

    // todo should we create when init?
    let map = GPUTexture::create(
      TextureDescriptor {
        label: "shadow-maps".into(),
        size: init_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        view_formats: &[],
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
      },
      &gpu.device,
    );
    let map = map.create_view(Default::default()).try_into().unwrap();

    let mapping = Default::default();

    let sampler = GPUSampler::create(
      SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Nearest,
        compare: CompareFunction::Greater.into(),
        ..Default::default()
      },
      &gpu.device,
    )
    .create_view(())
    .try_into()
    .unwrap();

    Self {
      id: 0,
      map,
      gpu,
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
  deltas: Box<dyn Stream<Item = ShadowMapAddressInfo> + Unpin>,
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
    let mut this = self.project();
    if let Poll::Ready(r) = this.deltas.as_mut().poll_next_unpin(cx) {
      if let Some(r) = r {
        *this.current = r;
      }
      Poll::Ready(r)
    } else {
      Poll::Pending
    }
  }
}

impl ShadowMap {
  pub fn get_write_view(&self) -> (GPU2DTextureView, ShadowMapAddressInfo) {
    let inner = self.inner.borrow();
    let allocation = inner.allocations.get(&self.id).unwrap();
    (
      inner
        .map
        .resource
        .create_view(TextureViewDescriptor {
          label: Some("shadow-write-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: allocation.info.layer_index as u32,
          array_layer_count: Some(1),
          ..Default::default()
        })
        .try_into()
        .unwrap(),
      allocation.info,
    )
  }
}
