use bytemuck::Pod;
use std::marker::PhantomData;

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
      usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
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
      min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<T>() as u64),
    }
  }
}
