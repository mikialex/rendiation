use std::{
  cell::Cell,
  rc::{Rc, Weak},
};

use rendiation_texture::TextureSampler;
use rendiation_webgpu::GPU;

use crate::*;

pub struct BindGroupDirtyWatcher {
  dirty: Cell<bool>,
}
impl Default for BindGroupDirtyWatcher {
  fn default() -> Self {
    Self {
      dirty: Cell::new(false),
    }
  }
}

impl BindGroupDirtyWatcher {
  pub fn notify_dirty(&self) {
    self.dirty.set(true);
  }

  pub fn get_dirty(&self) -> bool {
    self.dirty.get()
  }

  pub fn reset_clean(&self) {
    self.dirty.set(false)
  }
}

impl<T> Watcher<T> for Weak<BindGroupDirtyWatcher> {
  fn will_change(&mut self, _item: &T, _id: usize) -> bool {
    if let Some(v) = self.upgrade() {
      v.notify_dirty();
      return false;
    }
    true
  }

  fn will_drop(&mut self, _item: &T, _id: usize) {
    if let Some(v) = self.upgrade() {
      v.notify_dirty();
    }
  }
}

pub struct MaterialBindGroup<T = BindGroupDirtyWatcher> {
  pub gpu: Rc<wgpu::BindGroup>,
  pub dirty_notifier: Rc<T>,
}

pub struct MaterialBindGroupBuilder<'a, 'b> {
  device: &'a wgpu::Device,
  queue: &'a wgpu::Queue,
  bindings: Vec<wgpu::BindingResource<'b>>,
  dirty_notifier: Rc<BindGroupDirtyWatcher>,
  resources: &'a mut GPUResourceSubCache,
}

impl<'a, 'b> SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub fn map_sampler(
    &mut self,
    sampler: TextureSampler,
    device: &wgpu::Device,
  ) -> Rc<wgpu::Sampler> {
    self.resources.samplers.retrieve(device, &sampler)
  }
}

impl<'a, 'b> MaterialBindGroupBuilder<'a, 'b> {
  pub fn new(
    gpu: &'a GPU,
    resources: &'a mut GPUResourceSubCache,
    watcher: Rc<BindGroupDirtyWatcher>,
  ) -> Self {
    MaterialBindGroupBuilder {
      device: &gpu.device,
      queue: &gpu.queue,
      dirty_notifier: watcher,
      bindings: Vec::with_capacity(4),
      resources,
    }
  }

  pub fn push(mut self, binding: wgpu::BindingResource<'b>) -> Self {
    self.bindings.push(binding);
    self
  }

  pub fn push_texture<T>(mut self, texture: &'b SceneTexture<T>) -> Self
  where
    SceneTexture<T>: MaterialBindableResourceUpdate,
  {
    let bindable = texture.update(self.resources, self.device, self.queue);
    // I think this is safety! 🙏
    let bindable = unsafe { std::mem::transmute(bindable) };
    self.bindings.push(bindable);

    let weak = Rc::downgrade(&self.dirty_notifier);

    texture.content.borrow_mut().watchers.insert(Box::new(weak));

    self
  }

  pub fn build(self, layout: &wgpu::BindGroupLayout) -> MaterialBindGroup {
    let entries: Vec<_> = self
      .bindings
      .into_iter()
      .enumerate()
      .map(|(i, resource)| wgpu::BindGroupEntry {
        binding: i as u32,
        resource,
      })
      .collect();

    let gpu = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &entries,
      label: None,
    });

    let gpu = Rc::new(gpu);

    MaterialBindGroup {
      gpu,
      dirty_notifier: self.dirty_notifier,
    }
  }
}
