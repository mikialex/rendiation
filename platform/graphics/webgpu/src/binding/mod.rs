use rendiation_shader_api::ShaderBindingProvider;

use crate::*;

mod cache;
pub use cache::*;

mod bind_source;
pub use bind_source::*;

mod owned;
pub use owned::*;

pub trait BindableResourceProvider {
  fn get_bindable(&self) -> BindingResourceOwned;
}

pub trait BindableResourceView {
  fn as_bindable(&self) -> gpu::BindingResource<'_>;
}

#[derive(Clone)]
pub struct GPUBindGroupLayout {
  pub(crate) inner: gpu::BindGroupLayout,
  pub(crate) cache_id: u64,
}

impl Deref for GPUBindGroupLayout {
  type Target = gpu::BindGroupLayout;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

#[derive(Default)]
pub struct BindGroupBuilder {
  items: Vec<CacheAbleBindingBuildSource>,
}

impl BindGroupBuilder {
  pub fn iter_bounded(&self) -> impl Iterator<Item = &CacheAbleBindingBuildSource> {
    self.items.iter()
  }

  pub fn reset(&mut self) {
    self.items.clear();
  }

  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }

  pub fn bind(&mut self, source: CacheAbleBindingBuildSource) {
    self.items.push(source);
  }

  fn hash_binding_ids(&self, hasher: &mut impl Hasher) {
    self.items.iter().for_each(|b| {
      b.view_id.hash(hasher);
    });
  }
}

pub struct BindingBuilder {
  groups: [BindGroupBuilder; 5],
  checking_layouts: Option<Vec<Vec<ShaderBindingDescriptor>>>,
  current_index: usize,
  pub device: Option<GPUDevice>,
  pub any_states: FastHashMap<u64, Box<dyn Any>>,
}

impl Default for BindingBuilder {
  fn default() -> Self {
    Self {
      groups: std::array::from_fn(|_| BindGroupBuilder::default()),
      checking_layouts: Default::default(),
      any_states: Default::default(),
      current_index: 0,
      device: None,
    }
  }
}

pub trait AbstractPassBinding {
  fn set_bind_group_placeholder(&mut self, index: u32);
  fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup, offsets: &[DynamicOffset]);
}

impl BindingBuilder {
  pub fn new_with_device(device: GPUDevice) -> Self {
    Self {
      device: device.into(),
      ..Default::default()
    }
  }

  pub fn bind_immediate_sampler(
    &mut self,
    sampler: &(impl Into<gpu::SamplerDescriptor<'static>> + Clone),
  ) {
    let sampler_desc = sampler.clone().into();
    let is_compare = sampler_desc.compare.is_some();
    let sampler = GPUSampler::create(
      sampler_desc,
      self
        .device
        .as_ref()
        .expect("using sampler bind requires new_with_device"),
    );
    let sampler = sampler.create_default_view();
    if is_compare {
      let sampler = GPUComparisonSamplerView(sampler);
      self.bind(&sampler);
    } else {
      self.bind(&sampler);
    }
  }

  pub fn iter_groups(&self) -> impl Iterator<Item = &BindGroupBuilder> {
    self.groups.iter()
  }

  pub fn setup_checking_layout(&mut self, layouts: &[Vec<ShaderBindingDescriptor>]) {
    self.checking_layouts = Some(layouts.to_owned());
  }

  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn reset(&mut self) {
    self.groups.iter_mut().for_each(|item| item.reset());
    self.checking_layouts = None;
    self.any_states.clear();
  }

  pub fn with_bind<T>(mut self, item: &T) -> Self
  where
    T: AbstractBindingSource,
  {
    self.bind(item);
    self
  }

  pub fn with_fn(mut self, f: impl FnOnce(&mut Self)) -> Self {
    f(&mut self);
    self
  }

  pub fn bind<T>(&mut self, item: &T) -> &mut Self
  where
    T: AbstractBindingSource,
  {
    item.bind_pass(self);
    self
  }

  pub fn bind_single<T>(&mut self, item: &T) -> &mut Self
  where
    T: CacheAbleBindingSource + ShaderBindingProvider,
  {
    // check if the layout match, or panic directly, this is helpful to debug binding mismatch because the wgpu
    // validation is too late to catch where the miss match happens.
    if let Some(checking_layouts) = &mut self.checking_layouts {
      let desc = item.binding_desc();
      let layout = &checking_layouts[self.current_index];
      let target_idx = self.groups[self.current_index].items.len();

      fn is_layout_match(a: &ShaderBindingDescriptor, b: &ShaderBindingDescriptor) -> bool {
        let mut same = a == b;
        if !same {
          let mut a = a.clone();
          let mut b = b.clone();
          fn normalize(x: &mut ShaderBindingDescriptor) {
            if let ShaderValueType::Single(ShaderValueSingleType::Texture {
              sample_type: TextureSampleType::Float { filterable },
              ..
            }) = &mut x.ty
            {
              *filterable = false;
            }
            if let ShaderValueType::Single(ShaderValueSingleType::Sampler(s_ty)) = &mut x.ty {
              if let SamplerBindingType::NonFiltering | SamplerBindingType::Filtering = s_ty {
                *s_ty = SamplerBindingType::NonFiltering;
              }
            }
          }
          normalize(&mut a);
          normalize(&mut b);
          if a == b {
            same = true;
          }
        }
        same
      }

      if !is_layout_match(&desc, &layout[target_idx]) {
        panic!(
          "binding mismatch: \n binding is: \n {:#?}, \n pipeline expect: \n {:#?}",
          &desc, &layout[target_idx]
        );
      }
    }
    self.bind_dyn(item.get_binding_build_source())
  }

  pub fn bind_dyn(&mut self, source: CacheAbleBindingBuildSource) -> &mut Self {
    self.groups[self.current_index].bind(source);
    self
  }

  fn setup_binding<T: AbstractPassBinding>(
    &self,
    pass: &mut T,
    device: &GPUDevice,
    layouts: &[GPUBindGroupLayout],
  ) {
    let mut is_visiting_empty_tail = true;
    for (group_index, group) in self.groups.iter().enumerate().rev() {
      if group.is_empty() {
        if is_visiting_empty_tail {
          continue;
        } else {
          pass.set_bind_group_placeholder(group_index as u32);
        }
      }
      is_visiting_empty_tail = false;

      let layout = &layouts[group_index];

      // hash
      let mut hasher = FastHasher::default();
      group.hash_binding_ids(&mut hasher);
      layout.cache_id.hash(&mut hasher);
      let hash = hasher.finish();

      let cache = device.get_binding_cache();
      let mut binding_cache = cache.cache.write();

      let bindgroup = binding_cache.get_or_create(
        hash,
        || CacheAbleBindingBuildSource::build_bindgroup(group.items.as_slice(), device, layout),
        group.items.iter().map(|item| item.view_id),
      );

      pass.set_bind_group(group_index as u32, bindgroup, &[]);
    }
  }

  pub fn setup_compute_pass(
    self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
    pipeline: &GPUComputePipeline,
  ) {
    self.setup_binding(pass, device, &pipeline.raw_bg_layouts);
    pass.set_gpu_pipeline(pipeline);
  }

  pub fn setup_render_pass(
    &mut self,
    pass: &mut GPURenderPass,
    device: &GPUDevice,
    pipeline: &GPURenderPipeline,
  ) {
    self.setup_binding(pass, device, &pipeline.raw_bg_layouts);
    pass.set_gpu_pipeline(pipeline);
  }
}

pub trait AbstractBindingSource {
  fn bind_pass(&self, ctx: &mut BindingBuilder);
}

impl<T: CacheAbleBindingSource + ShaderBindingProvider> AbstractBindingSource for T {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind_single(self);
  }
}

pub struct AbstractShaderBindingIterSourceMap<T, F>(pub T, pub F);

impl<T, S, X, Y, F> AbstractShaderBindingSource for AbstractShaderBindingIterSourceMap<T, F>
where
  T: AbstractShaderBindingSource,
  T::ShaderBindResult: IntoShaderIterator<ShaderIter = S> + Clone,
  S: ShaderIterator<Item = X> + 'static,
  F: Fn(X) -> Y + Copy + 'static,
{
  type ShaderBindResult = ShaderIntoIterMap<T::ShaderBindResult, F>;

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    let inner = self.0.bind_shader(ctx);
    inner.map(self.1)
  }
}

impl<T, F> AbstractBindingSource for AbstractShaderBindingIterSourceMap<T, F>
where
  T: AbstractBindingSource,
{
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    self.0.bind_pass(ctx);
  }
}

pub struct AbstractShaderBindingIterSourceZip<L, S>(pub L, pub S);

impl<L, S> AbstractShaderBindingSource for AbstractShaderBindingIterSourceZip<L, S>
where
  L: AbstractShaderBindingSource,
  L::ShaderBindResult: IntoShaderIterator,
  S: AbstractShaderBindingSource,
  S::ShaderBindResult: IntoShaderIterator,
{
  type ShaderBindResult = ShaderIntoIterZip<L::ShaderBindResult, S::ShaderBindResult>;

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    self.0.bind_shader(ctx).zip(self.1.bind_shader(ctx))
  }
}

impl<L, S> AbstractBindingSource for AbstractShaderBindingIterSourceZip<L, S>
where
  L: AbstractBindingSource,
  S: AbstractBindingSource,
{
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    self.0.bind_pass(ctx);
    self.1.bind_pass(ctx);
  }
}
