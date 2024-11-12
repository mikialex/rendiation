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

#[derive(Default)]
pub struct BindGroupBuilder {
  items: Vec<CacheAbleBindingBuildSource>,
  // layouts: Vec<gpu::BindGroupLayoutEntry>,
}

impl std::fmt::Debug for BindGroupBuilder {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("BindGroupBuilder").finish()
  }
}

impl BindGroupBuilder {
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
  fn attach_bindgroup_invalidation_token(&self, mut token: BindGroupCacheInvalidation) {
    self.items.iter().for_each(|b| {
      b.source.increase(&token);
    });
    token.skip_drop = true;
  }
}

#[derive(Default)]
pub struct BindingBuilder {
  groups: [BindGroupBuilder; 5],
  current_index: usize,
}

impl BindingBuilder {
  pub fn new_as_compute() -> Self {
    let groups: Vec<_> = (0..5).map(|_| BindGroupBuilder::default()).collect();
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
    let desc = item.binding_desc();
    self.bind_dyn(item.get_binding_build_source())
  }

  pub fn bind_dyn(&mut self, source: CacheAbleBindingBuildSource) -> &mut Self {
    self.groups[self.current_index].bind(source);
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

        let bindgroup =
          CacheAbleBindingBuildSource::build_bindgroup(group.items.as_slice(), device, layout);
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

        let bindgroup =
          CacheAbleBindingBuildSource::build_bindgroup(group.items.as_slice(), device, layout);
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
