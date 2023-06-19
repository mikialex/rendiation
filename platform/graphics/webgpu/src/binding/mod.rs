use core::mem::ManuallyDrop;

use shadergraph::{ShaderGraphNodeType, ShaderUniformProvider};

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
  pub(crate) inner: Rc<gpu::BindGroupLayout>,
  pub(crate) cache_id: u64,
}

impl Deref for GPUBindGroupLayout {
  type Target = gpu::BindGroupLayout;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub trait CacheAbleBindingSource {
  fn get_uniform(&self) -> CacheAbleBindingBuildSource;
}

impl<T> CacheAbleBindingSource for ResourceViewRc<T>
where
  T: Resource,
  Self: BindableResourceProvider,
{
  fn get_uniform(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: self.get_bindable(),
      ref_increase_target: self.resource.bindgroup_holder.create_pending_increase(),
      view_id: self.guid,
    }
  }
}

pub struct CacheAbleBindingBuildSource {
  pub(crate) source: BindingResourceOwned,
  pub(crate) ref_increase_target: BindGroupResourcePendingIncrease,
  pub(crate) view_id: usize,
}

pub struct BindGroupBuilder<T> {
  items: Vec<T>,
  layouts: Vec<gpu::BindGroupLayoutEntry>,
}

impl<T> Default for BindGroupBuilder<T> {
  fn default() -> Self {
    Self {
      items: Default::default(),
      layouts: Default::default(),
    }
  }
}

pub trait BindingGroupBuildImpl: Sized {
  fn build_bindgroup(sources: &[Self], device: &GPUDevice, layout: &BindGroupLayout) -> BindGroup;
}

impl BindingGroupBuildImpl for CacheAbleBindingBuildSource {
  fn build_bindgroup(sources: &[Self], device: &GPUDevice, layout: &BindGroupLayout) -> BindGroup {
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
  pub fn reset(&mut self) {
    self.items.clear();
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
  pub fn bind<T>(&mut self, item: &T)
  where
    T: CacheAbleBindingSource + ShaderUniformProvider,
  {
    self.bind_raw(
      item.get_uniform(),
      map_shader_value_ty_to_binding_layout_type(
        <<T as ShaderUniformProvider>::Node as ShaderGraphNodeType>::TYPE,
        self.items.len(),
      ),
    )
  }
  fn hash_binding_ids(&self, hasher: &mut impl Hasher) {
    self.items.iter().for_each(|b| {
      b.view_id.hash(hasher);
    });
  }
  fn attach_bindgroup_invalidation_token(&self, token: BindGroupCacheInvalidation) {
    self.items.iter().for_each(|b| {
      b.ref_increase_target.increase(token.clone_another());
    });
    let _ = ManuallyDrop::new(token);
  }
}

#[derive(Default)]
pub struct BindingBuilder {
  groups: [BindGroupBuilder<CacheAbleBindingBuildSource>; 5],
  current_index: usize,
}

impl BindingBuilder {
  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn reset(&mut self) {
    self.groups.iter_mut().for_each(|item| item.reset());
  }

  pub fn bind<T>(&mut self, item: &T)
  where
    T: CacheAbleBindingSource + ShaderUniformProvider,
  {
    self.groups[self.current_index].bind(item)
  }

  pub fn setup_pass(
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
      let mut hasher = DefaultHasher::default();
      group.hash_binding_ids(&mut hasher);
      layout.cache_id.hash(&mut hasher);
      let hash = hasher.finish();

      let cache = device.get_binding_cache();
      let mut binding_cache = cache.cache.borrow_mut();

      let bindgroup = binding_cache.entry(hash).or_insert_with(|| {
        // build bindgroup and cache and return

        group.attach_bindgroup_invalidation_token(BindGroupCacheInvalidation {
          cache_id_to_drop: hash,
          cache: cache.clone(),
        });

        let bindgroup = group.create_bind_group(device, layout);
        Rc::new(bindgroup)
      });

      pass.set_bind_group_owned(group_index as u32, bindgroup, &[]);
    }
  }
}

impl<'encoder, 'gpu> GPURenderPassCtx<'encoder, 'gpu> {
  pub fn bind_immediate_sampler(
    &mut self,
    sampler: &(impl Into<SamplerDescriptor<'static>> + Clone),
  ) {
    let sampler = GPUSampler::create(sampler.clone().into(), &self.gpu.device);
    let sampler = sampler.create_default_view();
    self.binding.bind(&sampler);
  }
}

impl BindableResourceProvider for ResourceViewRc<RawSampler> {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::RawSampler(self.view.0.clone())
  }
}
impl BindableResourceProvider for ResourceViewRc<RawComparisonSampler> {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::RawSampler(self.view.0.clone())
  }
}