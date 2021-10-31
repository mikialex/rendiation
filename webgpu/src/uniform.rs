use bytemuck::Pod;
use std::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use super::BindableResource;

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

pub struct UniformBufferData<T> {
  gpu: wgpu::Buffer,
  data: T,
  changed: bool,
}

impl<T> Deref for UniformBufferData<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl<T> DerefMut for UniformBufferData<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.changed = true;
    &mut self.data
  }
}

impl<T: Pod> UniformBufferData<T> {
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
      changed: false,
    }
  }

  pub fn update(&self, queue: &wgpu::Queue) {
    if self.changed {
      queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[self.data]))
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
