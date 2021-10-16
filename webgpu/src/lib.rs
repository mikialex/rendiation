#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod buffer;
mod cache;
mod encoder;
mod queue;
mod render_target;
mod sampler;
mod surface;
mod texture;
mod uniform;

use std::cell::RefCell;

pub use cache::*;
pub use encoder::*;
pub use queue::*;
pub use render_target::*;
pub use sampler::*;
pub use surface::*;
pub use texture::*;
pub use uniform::*;

pub use wgpu::*;

use bytemuck::Pod;
use rendiation_texture_types::Size;

pub struct If<const B: bool>;
pub trait True {}
impl True for If<true> {}
pub trait True2 {}
impl True2 for If<true> {}

pub trait BindableResource {
  fn as_bindable(&self) -> wgpu::BindingResource;
  fn bind_layout() -> wgpu::BindingType;
}

impl BindableResource for wgpu::Sampler {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::Sampler(self)
  }
  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Sampler {
      comparison: false,
      filtering: true,
    }
  }
}

pub trait Renderable {
  fn update(&mut self, gpu: &GPU, encoder: &mut wgpu::CommandEncoder) {
    // assume all gpu stuff prepared, and do nothing
  }
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}

pub trait RenderPassCreator<T> {
  fn create<'a>(
    &'a self,
    target: &'a T,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a>;

  // fn get_color_formats(&self) -> Vec<wgpu::TextureFormat>;
  // fn get_depth_stencil_format(&self) -> Option<wgpu::TextureFormat>;
}

pub struct GPU {
  instance: wgpu::Instance,
  adaptor: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub encoder: RefCell<wgpu::CommandEncoder>,
}

pub trait SurfaceProvider {
  fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface;
  fn size(&self) -> Size;
}

impl SurfaceProvider for winit::window::Window {
  fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface {
    unsafe { instance.create_surface(self) }
  }

  fn size(&self) -> Size {
    let size = self.inner_size();
    Size::from_u32_pair_min_one((size.width, size.height))
  }
}

impl GPU {
  pub async fn new() -> Self {
    let backend = wgpu::Backends::PRIMARY;
    let instance = wgpu::Instance::new(backend);
    let power_preference = wgpu::PowerPreference::default();

    let adaptor = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: None,
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adaptor
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await
      .expect("Unable to find a suitable GPU device!");

    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: "Main GPU encoder".into(),
    });

    let encoder = RefCell::new(encoder);

    Self {
      instance,
      adaptor,
      device,
      queue,
      encoder,
    }
  }
  pub async fn new_with_surface(surface_provider: &dyn SurfaceProvider) -> (Self, GPUSurface) {
    let backend = wgpu::Backends::PRIMARY;
    let instance = wgpu::Instance::new(backend);
    let power_preference = wgpu::PowerPreference::default();

    let surface = surface_provider.create_surface(&instance);
    let size = surface_provider.size();

    let adaptor = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: Some(&surface),
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adaptor
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await
      .expect("Unable to find a suitable GPU device!");

    let surface = GPUSurface::new(&adaptor, &device, surface, size);

    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: "Main GPU encoder".into(),
    });

    let encoder = RefCell::new(encoder);

    (
      Self {
        instance,
        adaptor,
        device,
        queue,
        encoder,
      },
      surface,
    )
  }

  pub fn submit(&self) {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: "Main GPU encoder".into(),
      });

    let mut current_encoder = self.encoder.borrow_mut();
    let current_encoder: &mut wgpu::CommandEncoder = &mut current_encoder;

    std::mem::swap(current_encoder, &mut encoder);

    self.queue.submit(Some(encoder.finish()));
  }

  pub fn render_pass<R, T>(&self, renderable: &mut R, target: &T)
  where
    R: Renderable,
    R: RenderPassCreator<T>,
  {
    let mut encoder = self.encoder.borrow_mut();
    {
      renderable.update(self, &mut encoder);
      let mut pass = renderable.create(target, &mut encoder);
      renderable.setup_pass(&mut pass);
    }
  }
}

pub trait VertexBufferSourceType {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static>;
  fn get_shader_header() -> &'static str;
}

pub trait IndexBufferSourceType: Pod {
  const FORMAT: wgpu::IndexFormat;
}

impl IndexBufferSourceType for u32 {
  const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

impl IndexBufferSourceType for u16 {
  const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}
