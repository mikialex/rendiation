use core::mem::ManuallyDrop;

use shadergraph::{ShaderGraphNodeType, ShaderUniformProvider, ShaderValueType};

use crate::*;

pub trait BindableResourceView {
  fn as_bindable(&self) -> gpu::BindingResource;
}

#[derive(Clone)]
pub struct BindGroupCache {
  cache: Rc<RefCell<HashMap<u64, Rc<gpu::BindGroup>>>>,
}
impl BindGroupCache {
  pub(crate) fn new() -> Self {
    Self {
      cache: Default::default(),
    }
  }
}

#[derive(Clone, Default)]
pub struct BindGroupLayoutCache {
  pub cache: Rc<RefCell<HashMap<u64, GPUBindGroupLayout>>>,
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

pub struct BindGroupCacheInvalidation {
  cache_id_to_drop: u64,
  cache: BindGroupCache,
}

impl Drop for BindGroupCacheInvalidation {
  fn drop(&mut self) {
    self.cache.cache.borrow_mut().remove(&self.cache_id_to_drop);
  }
}

pub trait BindProvider: BindableResourceView + 'static {
  fn view_id(&self) -> usize;
  fn add_bind_record(&self, record: BindGroupCacheInvalidation);
}

pub trait BindingSource {
  type Uniform: BindProvider;
  fn get_uniform(&self) -> Self::Uniform;
}

impl<T> BindingSource for ResourceViewRc<T>
where
  T: Resource,
  T::View: BindableResourceView,
{
  type Uniform = Self;

  fn get_uniform(&self) -> Self::Uniform {
    self.clone()
  }
}

#[derive(Default)]
pub struct BindGroupBuilder {
  items: Vec<(Box<dyn BindProvider>, ShaderValueType)>,
}

impl BindGroupBuilder {
  pub fn reset(&mut self) {
    self.items.clear();
  }

  pub fn bind<T>(&mut self, item: &T)
  where
    T: BindingSource + ShaderUniformProvider,
  {
    self.items.push((
      Box::new(item.get_uniform()),
      <<T as ShaderUniformProvider>::Node as ShaderGraphNodeType>::TYPE,
    ))
  }

  pub fn create_bind_group_layout(&mut self, device: &GPUDevice) -> GPUBindGroupLayout {
    create_bindgroup_layout_by_node_ty(device, self.items.iter().map(|v| &v.1))
  }

  pub fn create_bind_group(
    &mut self,
    device: &GPUDevice,
    layout: &wgpu::BindGroupLayout,
  ) -> wgpu::BindGroup {
    let entries: Vec<_> = self
      .items
      .iter()
      .enumerate()
      .map(|(i, item)| unsafe {
        gpu::BindGroupEntry {
          binding: i as u32,
          resource: std::mem::transmute(item.0.as_bindable()),
        }
      })
      .collect();

    device.create_bind_group(&gpu::BindGroupDescriptor {
      label: None,
      layout,
      entries: &entries,
    })
  }

  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }

  fn hash_binding_ids(&self, hasher: &mut impl Hasher) {
    self.items.iter().for_each(|b| {
      b.0.view_id().hash(hasher);
    });
  }

  fn attach_bindgroup_invalidation_token(&self, token: BindGroupCacheInvalidation) {
    self.items.iter().for_each(|b| {
      // note to be careful, we do not impl clone
      b.0.add_bind_record(BindGroupCacheInvalidation {
        cache_id_to_drop: token.cache_id_to_drop,
        cache: token.cache.clone(),
      });
    });
    let _ = ManuallyDrop::new(token);
  }
}

#[derive(Default)]
pub struct BindingBuilder {
  groups: [BindGroupBuilder; 5],
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
    T: BindingSource + ShaderUniformProvider,
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
