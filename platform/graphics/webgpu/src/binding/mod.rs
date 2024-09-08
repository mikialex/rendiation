use rendiation_shader_api::ShaderBindingProvider;

use crate::*;

mod cache;
pub use cache::*;

mod owned;
pub use owned::*;

pub trait BindableResourceProvider {
  fn get_bindable(&self) -> BindingResourceOwned;
}

pub trait BindableResourceView {
  fn as_bindable(&self) -> gpu::BindingResource;
}

#[derive(Clone)]
pub struct GPUBindGroupLayout {
  pub(crate) inner: Arc<gpu::BindGroupLayout>,
  pub(crate) cache_id: u64,
}

impl Deref for GPUBindGroupLayout {
  type Target = gpu::BindGroupLayout;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub trait CacheAbleBindingSource {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource;
}

impl<T> CacheAbleBindingSource for ResourceViewRc<T>
where
  T: Resource,
  Self: BindableResourceProvider,
{
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: self.get_bindable(),
      view_id: self.guid,
    }
  }
}

pub struct CacheAbleBindingBuildSource {
  pub(crate) source: BindingResourceOwned,
  pub(crate) view_id: usize,
}

pub struct BindGroupBuilder<T> {
  is_compute: bool,
  items: Vec<T>,
  layouts: Vec<gpu::BindGroupLayoutEntry>,
}

impl<T> std::fmt::Debug for BindGroupBuilder<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("BindGroupBuilder")
      .field("is_compute", &self.is_compute)
      .finish()
  }
}

impl<T> Default for BindGroupBuilder<T> {
  fn default() -> Self {
    Self {
      is_compute: false,
      items: Default::default(),
      layouts: Default::default(),
    }
  }
}

pub trait BindingGroupBuildImpl: Sized {
  fn build_bindgroup(
    sources: &[Self],
    device: &GPUDevice,
    layout: &gpu::BindGroupLayout,
  ) -> gpu::BindGroup;
}

impl BindingGroupBuildImpl for CacheAbleBindingBuildSource {
  fn build_bindgroup(
    sources: &[Self],
    device: &GPUDevice,
    layout: &gpu::BindGroupLayout,
  ) -> gpu::BindGroup {
    let entries_prepare: Vec<_> = sources.iter().map(|s| s.source.prepare_ref()).collect();
    let entries: Vec<_> = entries_prepare
      .iter()
      .enumerate()
      .map(|(i, s)| gpu::BindGroupEntry {
        binding: i as u32,
        resource: s.as_binding_ref(),
      })
      .collect();

    device.create_bind_group(&gpu::BindGroupDescriptor {
      label: None,
      layout,
      entries: &entries,
    })
  }
}

impl<T> BindGroupBuilder<T> {
  pub fn new_as_compute() -> Self {
    Self {
      is_compute: true,
      ..Default::default()
    }
  }

  pub fn reset(&mut self) {
    self.items.clear();
    self.layouts.clear();
  }

  pub fn bind_raw(&mut self, item: T, entry_ty: gpu::BindGroupLayoutEntry) {
    self.items.push(item);
    self.layouts.push(entry_ty);
  }

  pub fn create_bind_group_layout(&mut self, device: &GPUDevice) -> GPUBindGroupLayout {
    device.create_and_cache_bindgroup_layout(self.layouts.as_ref())
  }

  pub fn create_bind_group(
    &mut self,
    device: &GPUDevice,
    layout: &wgpu::BindGroupLayout,
  ) -> wgpu::BindGroup
  where
    T: BindingGroupBuildImpl,
  {
    T::build_bindgroup(&self.items, device, layout)
  }

  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }
}

impl BindGroupBuilder<CacheAbleBindingBuildSource> {
  pub fn bind(&mut self, source: CacheAbleBindingBuildSource, desc: &ShaderBindingDescriptor) {
    self.bind_raw(
      source,
      map_shader_value_ty_to_binding_layout_type(desc, self.items.len(), self.is_compute),
    )
  }
  fn hash_binding_ids(&self, hasher: &mut impl Hasher) {
    self.items.iter().for_each(|b| {
      b.view_id.hash(hasher);
    });
  }
  fn attach_bindgroup_invalidation_token(&self, mut token: BindGroupCacheInvalidation) {
    self.items.iter().for_each(|b| {
      b.source.increase(&token);
    });
    token.skip_drop = true;
  }
}

#[derive(Default)]
pub struct BindingBuilder {
  groups: [BindGroupBuilder<CacheAbleBindingBuildSource>; 5],
  current_index: usize,
}

impl BindingBuilder {
  pub fn new_as_compute() -> Self {
    let groups: Vec<_> = (0..5).map(|_| BindGroupBuilder::new_as_compute()).collect();
    Self {
      groups: groups.try_into().unwrap(),
      ..Default::default()
    }
  }

  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn reset(&mut self) {
    self.groups.iter_mut().for_each(|item| item.reset());
  }

  pub fn with_bind<T>(mut self, item: &T) -> Self
  where
    T: CacheAbleBindingSource + ShaderBindingProvider,
  {
    self.bind(item);
    self
  }

  pub fn bind<T>(&mut self, item: &T) -> &mut Self
  where
    T: CacheAbleBindingSource + ShaderBindingProvider,
  {
    self.bind_dyn(item.get_binding_build_source(), &item.binding_desc())
  }

  pub fn bind_dyn(
    &mut self,
    source: CacheAbleBindingBuildSource,
    desc: &ShaderBindingDescriptor,
  ) -> &mut Self {
    self.groups[self.current_index].bind(source, desc);
    self
  }

  pub fn setup_render_pass(
    &mut self,
    pass: &mut GPURenderPass,
    device: &GPUDevice,
    pipeline: &GPURenderPipeline,
  ) {
    let mut is_visiting_empty_tail = true;
    for (group_index, group) in self.groups.iter_mut().enumerate().rev() {
      if group.is_empty() {
        if is_visiting_empty_tail {
          continue;
        } else {
          pass.set_bind_group_placeholder(group_index as u32);
        }
      }
      is_visiting_empty_tail = false;

      let layout = &pipeline.bg_layouts[group_index];

      // hash
      let mut hasher = FastHasher::default();
      group.hash_binding_ids(&mut hasher);
      layout.cache_id.hash(&mut hasher);
      let hash = hasher.finish();

      let cache = device.get_binding_cache();
      let mut binding_cache = cache.cache.write().unwrap();

      let bindgroup = binding_cache.entry(hash).or_insert_with(|| {
        // build bindgroup and cache and return

        group.attach_bindgroup_invalidation_token(BindGroupCacheInvalidation {
          cache_id_to_drop: hash,
          cache: cache.clone(),
          skip_drop: false,
        });

        let bindgroup = group.create_bind_group(device, layout);
        Arc::new(bindgroup)
      });

      pass.set_bind_group(group_index as u32, bindgroup, &[]);
    }
    pass.set_gpu_pipeline(pipeline);
  }

  // todo, code reuse
  pub fn setup_compute_pass(
    mut self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
    pipeline: &GPUComputePipeline,
  ) {
    let mut is_visiting_empty_tail = true;
    for (group_index, group) in self.groups.iter_mut().enumerate().rev() {
      if group.is_empty() {
        if is_visiting_empty_tail {
          continue;
        } else {
          pass.set_bind_group_placeholder(group_index as u32);
        }
      }
      is_visiting_empty_tail = false;

      let layout = &pipeline.bg_layouts[group_index];

      // hash
      let mut hasher = FastHasher::default();
      group.hash_binding_ids(&mut hasher);
      layout.cache_id.hash(&mut hasher);
      let hash = hasher.finish();

      let cache = device.get_binding_cache();
      let mut binding_cache = cache.cache.write().unwrap();

      let bindgroup = binding_cache.entry(hash).or_insert_with(|| {
        // build bindgroup and cache and return

        group.attach_bindgroup_invalidation_token(BindGroupCacheInvalidation {
          cache_id_to_drop: hash,
          cache: cache.clone(),
          skip_drop: false,
        });

        let bindgroup = group.create_bind_group(device, layout);
        Arc::new(bindgroup)
      });

      pass.set_bind_group(group_index as u32, bindgroup, &[]);
    }
    pass.set_gpu_pipeline(pipeline);
  }
}

pub trait AbstractBindingSource {
  type ShaderBindResult;
  fn bind_pass(&self, ctx: &mut BindingBuilder);
  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult;
}

impl<T: CacheAbleBindingSource + ShaderBindingProvider> AbstractBindingSource for T {
  type ShaderBindResult = Node<T::Node>;

  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(self);
  }

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    ctx.bind_by(self)
  }
}
