use crate::*;

impl BindableResourceView for gpu::Sampler {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::Sampler(self)
  }
}

#[derive(Clone)]
pub struct RawSampler(pub Arc<gpu::Sampler>);

impl BindableResourceView for RawSampler {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::Sampler(self.0.as_ref())
  }
}

pub type GPUSampler = ResourceRc<RawSampler>;
pub type GPUSamplerView = ResourceViewRc<RawSampler>;

impl Resource for RawSampler {
  type Descriptor = gpu::SamplerDescriptor<'static>;

  type View = RawSampler;

  type ViewDescriptor = ();

  fn create_view(&self, _: &Self::ViewDescriptor) -> Self::View {
    self.clone()
  }
}

impl InitResourceByAllocation for RawSampler {
  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    device.create_and_cache_sampler(desc.clone())
  }
}

impl GPURenderPassCtx {
  pub fn bind_immediate_sampler(
    &mut self,
    sampler: &(impl Into<gpu::SamplerDescriptor<'static>> + Clone),
  ) {
    let sampler_desc = sampler.clone().into();
    let is_compare = sampler_desc.compare.is_some();
    let sampler = GPUSampler::create(sampler_desc, &self.gpu.device);
    let sampler = sampler.create_default_view();
    if is_compare {
      let sampler = GPUComparisonSamplerView(sampler);
      self.binding.bind(&sampler);
    } else {
      self.binding.bind(&sampler);
    }
  }
}

impl BindableResourceProvider for GPUSamplerView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::Sampler(self.clone())
  }
}

pub struct GPUComparisonSampler(pub GPUSampler);
pub struct GPUComparisonSamplerView(pub GPUSamplerView);

impl BindableResourceProvider for GPUComparisonSamplerView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::Sampler(self.0.clone())
  }
}

impl CacheAbleBindingSource for GPUComparisonSamplerView {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.0.get_binding_build_source()
  }
}

impl Deref for GPUComparisonSamplerView {
  type Target = GPUSamplerView;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl TryFrom<GPUSamplerView> for GPUComparisonSamplerView {
  type Error = &'static str;

  fn try_from(view: GPUSamplerView) -> Result<Self, Self::Error> {
    if view.resource.desc.compare.is_some() {
      Ok(Self(view))
    } else {
      Err("not comparison sampler")
    }
  }
}

/// make desc hashable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GPUSamplerDescriptor {
  /// How to deal with out of bounds accesses in the u (i.e. x) direction
  pub address_mode_u: AddressMode,
  /// How to deal with out of bounds accesses in the v (i.e. y) direction
  pub address_mode_v: AddressMode,
  /// How to deal with out of bounds accesses in the w (i.e. z) direction
  pub address_mode_w: AddressMode,
  /// How to filter the texture when it needs to be magnified (made larger)
  pub mag_filter: FilterMode,
  /// How to filter the texture when it needs to be minified (made smaller)
  pub min_filter: FilterMode,
  /// How to filter between mip map levels
  pub mipmap_filter: FilterMode,
  /// Minimum level of detail (i.e. mip level) to use
  pub lod_min_clamp: u32,
  /// Maximum level of detail (i.e. mip level) to use
  pub lod_max_clamp: u32,
  /// If this is enabled, this is a comparison sampler using the given comparison function.
  pub compare: Option<CompareFunction>,
  /// Valid values: 1, 2, 4, 8, and 16.
  pub anisotropy_clamp: u16,
  /// Border color to use when address_mode is [`AddressMode::ClampToBorder`]
  pub border_color: Option<SamplerBorderColor>,
}

impl From<GPUSamplerDescriptor> for gpu::SamplerDescriptor<'_> {
  fn from(s: GPUSamplerDescriptor) -> Self {
    Self {
      label: None,
      lod_min_clamp: f32::from_bits(s.lod_min_clamp),
      lod_max_clamp: f32::from_bits(s.lod_max_clamp),
      address_mode_u: s.address_mode_u,
      address_mode_v: s.address_mode_v,
      address_mode_w: s.address_mode_w,
      mag_filter: s.mag_filter,
      min_filter: s.min_filter,
      mipmap_filter: s.mipmap_filter,
      compare: s.compare,
      anisotropy_clamp: s.anisotropy_clamp,
      border_color: s.border_color,
    }
  }
}

impl<'a> From<gpu::SamplerDescriptor<'a>> for GPUSamplerDescriptor {
  fn from(s: gpu::SamplerDescriptor<'a>) -> Self {
    Self {
      lod_min_clamp: s.lod_min_clamp.to_bits(),
      lod_max_clamp: s.lod_max_clamp.to_bits(),
      address_mode_u: s.address_mode_u,
      address_mode_v: s.address_mode_v,
      address_mode_w: s.address_mode_w,
      mag_filter: s.mag_filter,
      min_filter: s.min_filter,
      mipmap_filter: s.mipmap_filter,
      compare: s.compare,
      anisotropy_clamp: s.anisotropy_clamp,
      border_color: s.border_color,
    }
  }
}

pub struct ImmediateGPUSamplerViewBind;

impl ShaderBindingProvider for ImmediateGPUSamplerViewBind {
  type Node = <GPUSamplerView as ShaderBindingProvider>::Node;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }
}

pub struct ImmediateGPUCompareSamplerViewBind;

impl ShaderBindingProvider for ImmediateGPUCompareSamplerViewBind {
  type Node = <GPUComparisonSamplerView as ShaderBindingProvider>::Node;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }
}
