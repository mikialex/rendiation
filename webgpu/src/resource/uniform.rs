use bytemuck::Pod;
use std::{
  cell::Cell,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use crate::BindableResource;

/// Typed uniform buffer
pub struct UniformBuffer<T> {
  gpu: wgpu::Buffer,
  phantom: PhantomData<T>,
}

impl<T: Pod> UniformBuffer<T> {
  pub fn create(device: &wgpu::Device, data: T) -> Self {
    use wgpu::util::DeviceExt;
    let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu,
      phantom: PhantomData,
    }
  }

  pub fn update(&self, queue: &wgpu::Queue, data: T) {
    queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[data]))
  }

  pub fn gpu(&self) -> &wgpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResource for UniformBuffer<T> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.gpu.as_entire_binding()
  }

  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
      ty: wgpu::BufferBindingType::Uniform,
      has_dynamic_offset: false,
      // min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<T>() as u64), // todo
      min_binding_size: None,
    }
  }
}

/// Typed uniform buffer with cpu data
pub struct UniformBufferData<T> {
  gpu: wgpu::Buffer,
  data: T,
  changed: Cell<bool>,
}

impl<T> Deref for UniformBufferData<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl<T> DerefMut for UniformBufferData<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.changed.set(true);
    &mut self.data
  }
}

impl<T: Pod> UniformBufferData<T> {
  pub fn create_default(device: &wgpu::Device) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &wgpu::Device, data: T) -> Self {
    use wgpu::util::DeviceExt;
    let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu,
      data,
      changed: Cell::new(false),
    }
  }

  pub fn update(&self, queue: &wgpu::Queue) {
    if self.changed.get() {
      queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[self.data]));
      self.changed.set(false);
    }
  }

  pub fn gpu(&self) -> &wgpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResource for UniformBufferData<T> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.gpu.as_entire_binding()
  }

  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
      ty: wgpu::BufferBindingType::Uniform,
      has_dynamic_offset: false,
      min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<T>() as u64),
    }
  }
}

pub struct UniformBufferDataWithCache<T> {
  gpu: wgpu::Buffer,
  data: T,
  last: Cell<Option<T>>, // maybe unsafe cell?
}

impl<T> Deref for UniformBufferDataWithCache<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl<T: Copy> DerefMut for UniformBufferDataWithCache<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    if self.last.get().is_none() {
      self.last.set(self.data.into())
    }
    &mut self.data
  }
}

impl<T: Pod> UniformBufferDataWithCache<T> {
  pub fn create_default(device: &wgpu::Device) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &wgpu::Device, data: T) -> Self {
    use wgpu::util::DeviceExt;
    let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu,
      data,
      last: Cell::new(None),
    }
  }

  pub fn update(&self, queue: &wgpu::Queue)
  where
    T: PartialEq,
  {
    if let Some(last) = self.last.get() {
      if last != self.data {
        queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[self.data]))
      }
      self.last.set(None)
    }
  }

  pub fn gpu(&self) -> &wgpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResource for UniformBufferDataWithCache<T> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.gpu.as_entire_binding()
  }

  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
      ty: wgpu::BufferBindingType::Uniform,
      has_dynamic_offset: false,
      min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<T>() as u64),
    }
  }
}
