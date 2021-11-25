use std::rc::Rc;

use rendiation_texture::TextureSampler;
use rendiation_webgpu::{BindableResource, WebGPUTexture2d, GPU};

use crate::*;

pub trait BindGroupDirtyNotifier: 'static {
  fn notify_dirty(&self);
}

pub struct MaterialBindGroup<T = BindGroupDirtyWatcher> {
  pub gpu: Rc<wgpu::BindGroup>,
  pub dirty_notifier: Rc<T>,
}

pub struct MaterialBindGroupBuilder<'a, 'b, T> {
  device: &'a wgpu::Device,
  queue: &'a wgpu::Queue,
  bindings: Vec<wgpu::BindingResource<'b>>,
  dirty_notifier: Rc<T>,
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

impl<'a, 'b, W: BindGroupDirtyNotifier> MaterialBindGroupBuilder<'a, 'b, W> {
  pub fn new(gpu: &'a GPU, watcher: Rc<W>) -> Self {
    MaterialBindGroupBuilder {
      device: &gpu.device,
      queue: &gpu.queue,
      dirty_notifier: watcher,
      bindings: Vec::with_capacity(4),
    }
  }

  pub fn push(mut self, binding: wgpu::BindingResource<'b>) -> Self {
    self.bindings.push(binding);
    self
  }

  pub fn push_texture<T, U>(mut self, texture: &'b SceneTexture<T, U>) -> Self
  where
    T: MaterialBindableResourceUpdate<GPU = U>,
  {
    let mut texture = texture.content.borrow_mut();

    let weak = Rc::downgrade(&self.dirty_notifier);

    let notifier = move || {
      if let Some(notifier) = weak.upgrade() {
        notifier.notify_dirty();
        return true;
      }

      false
    };

    texture.on_changed.push(Box::new(notifier));

    // this unsafe is ok, but could be improved
    let gpu: &'b WebGPUTexture2d = unsafe {
      let t: &mut SceneTextureContent<T, U> = &mut texture;
      let source = &t.source;
      let g = &mut t.gpu;
      let gpu = source.update(g, self.device, self.queue);
      std::mem::transmute(gpu)
    };

    self.bindings.push(gpu.as_bindable());

    self
  }

  pub fn build(self, layout: &wgpu::BindGroupLayout) -> MaterialBindGroup<W> {
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
