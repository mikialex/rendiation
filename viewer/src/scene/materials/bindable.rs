use std::cell::RefCell;

use crate::{
  renderer::BindableResource,
  scene::{MaterialHandle, ResourcePair},
};

pub struct MaterialBindableResource<T> {
  gpu: Option<T>,
  used_by: RefCell<Vec<MaterialHandle>>,
}

impl<T> Default for MaterialBindableResource<T> {
  fn default() -> Self {
    Self {
      gpu: None,
      used_by: RefCell::new(Vec::new()),
    }
  }
}

impl<T: BindableResource> MaterialBindableResource<T> {
  pub fn as_material_bind(&self, material: MaterialHandle) -> wgpu::BindingResource {
    self.used_by.borrow_mut().push(material);
    self.gpu.as_ref().unwrap().as_bindable()
  }
}

impl<T> MaterialBindableResource<T> {
  pub fn remove_material_bind(&self, material: MaterialHandle) {
    let index = self
      .used_by
      .borrow_mut()
      .iter()
      .position(|&h| h == material)
      .unwrap();
    self.used_by.borrow_mut().swap_remove(index);
  }

  pub fn update_gpu(&mut self) -> &mut Option<T> {
    &mut self.gpu
  }

  pub fn foreach_material_refed(&self, f: impl FnMut(MaterialHandle)) {
    self.used_by.borrow().iter().map(|&h| h).for_each(f)
  }
}

pub struct MaterialBindableItemPair<T, S> {
  data: T,
  res: MaterialBindableResource<S>,
}

impl<T, S> ResourcePair for MaterialBindableItemPair<T, S> {
  type Data = T;
  type Resource = MaterialBindableResource<S>;
  fn data(&self) -> &Self::Data {
    &self.data
  }
  fn resource(&self) -> &Self::Resource {
    &self.res
  }
  fn data_mut(&mut self) -> &mut Self::Data {
    *self.res.update_gpu() = None;
    &mut self.data
  }
  fn resource_mut(&mut self) -> &mut Self::Resource {
    &mut self.res
  }
}

pub trait MaterialBindableResourceUpdate {
  type GPU;
  fn update(&self, gpu: &mut Option<Self::GPU>, device: &wgpu::Device, queue: &wgpu::Queue);
}

impl<T: MaterialBindableResourceUpdate<GPU = S>, S> MaterialBindableItemPair<T, S> {
  pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    self.data.update(self.res.update_gpu(), device, queue);
  }
}

impl<T, S> MaterialBindableItemPair<T, S> {
  pub fn new(data: T) -> Self {
    Self {
      data,
      res: Default::default(),
    }
  }
  pub fn foreach_material_refed(&self, f: impl FnMut(MaterialHandle)) {
    self.res.foreach_material_refed(f)
  }
}
