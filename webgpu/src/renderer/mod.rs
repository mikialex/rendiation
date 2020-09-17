use std::{any::TypeId, cell::RefCell, collections::HashMap, sync::Arc};

pub mod bindgroup;
pub mod blend;
pub mod buffer;
pub mod pipeline;
pub mod render_pass;
pub mod render_target;
pub mod sampler;
pub mod shader_util;
pub mod swap_chain;
pub mod texture;

pub use bindgroup::*;
pub use blend::*;
pub use buffer::*;
pub use pipeline::*;
pub use render_pass::*;
pub use render_target::*;
pub use sampler::*;
pub use shader_util::*;
pub use swap_chain::*;
pub use texture::*;
pub use texture_dimension::*;

/// The renderer trait.
///
/// Impl this trait for build your own renderer.
pub trait Renderer: 'static + Sized {
  fn init(device: &wgpu::Device, size: (usize, usize)) -> Self;
  fn resize(&mut self, device: &wgpu::Device, size: (usize, usize));
  fn render();
}

/// WebGPU renderer backend
///
/// the backend render not contains any specific render resource.
/// just encapsulate webgpu functionality
pub struct WGPURenderer {
  pub instance: wgpu::Instance,
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub encoder: wgpu::CommandEncoder,
  pub swap_chain_format: wgpu::TextureFormat,
  pub bindgroup_layout_cache: RefCell<HashMap<TypeId, Arc<wgpu::BindGroupLayout>>>,
}

impl WGPURenderer {
  pub async fn new(instance: wgpu::Instance, surface: &wgpu::Surface) -> Self {
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: Some(surface),
      })
      .await
      .unwrap();

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await
      .unwrap();

    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    Self {
      instance,
      adapter,
      device,
      queue,
      encoder,
      swap_chain_format: wgpu::TextureFormat::Bgra8UnormSrgb,
      bindgroup_layout_cache: RefCell::new(HashMap::new()),
    }
  }

  pub fn submit(&mut self) {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    use std::mem;

    mem::swap(&mut encoder, &mut self.encoder);

    let command_buf = encoder.finish();
    let command_buf = vec![command_buf]; // todo avoid allocation
    self.queue.submit(command_buf);
  }

  pub fn register_bindgroup<T: WGPUBindGroupLayoutProvider>(&self) -> Arc<wgpu::BindGroupLayout> {
    let id = TypeId::of::<T>();
    let mut cache = self.bindgroup_layout_cache.borrow_mut();
    cache
      .entry(id)
      .or_insert_with(|| Arc::new(T::provide_layout(self)))
      .clone()
  }
}
