#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(const_fn_transmute)]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use self::swap_chain::SwapChain;
mod buffer;
mod encoder;
mod queue;
mod sampler;
mod swap_chain;
mod texture;
mod uniform;

pub use encoder::*;
pub use queue::*;
use rendiation_texture::Size;
pub use sampler::*;
pub use texture::*;
pub use uniform::*;

pub use wgpu::*;

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
  fn update(&mut self, gpu: &mut GPU, encoder: &mut wgpu::CommandEncoder) {
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
  swap_chain: SwapChain,
}

impl GPU {
  pub async fn new(window: &winit::window::Window) -> Self {
    let backend = wgpu::BackendBit::PRIMARY;
    let instance = wgpu::Instance::new(backend);
    let power_preference = wgpu::PowerPreference::default();

    let (size, surface) = unsafe {
      let size = window.inner_size();
      let surface = instance.create_surface(window);
      (size, surface)
    };
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

    let swap_chain = SwapChain::new(
      &adaptor,
      &device,
      surface,
      Size::from_u32_pair_min_one((size.width, size.height)),
    );

    Self {
      instance,
      adaptor,
      device,
      queue,
      swap_chain,
    }
  }
  pub fn render<R, T>(&mut self, renderable: &mut R, target: &T)
  where
    R: Renderable,
    R: RenderPassCreator<T>,
  {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
      renderable.update(self, &mut encoder);
      let mut pass = renderable.create(target, &mut encoder);
      renderable.setup_pass(&mut pass);
    }

    self.queue.submit(Some(encoder.finish()));
  }
  pub fn resize(&mut self, size: Size) {
    self.swap_chain.resize(size, &self.device);
  }

  pub fn create_shader_flags(&self) -> wgpu::ShaderFlags {
    let mut flags = wgpu::ShaderFlags::VALIDATION;
    match self.adaptor.get_info().backend {
      wgpu::Backend::Metal | wgpu::Backend::Vulkan => {
        flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
      }
      _ => (), //TODO
    }
    flags
  }
  pub fn get_prefer_target_format(&self) -> wgpu::TextureFormat {
    self.swap_chain.swap_chain_descriptor.format
  }

  pub fn get_current_frame(&mut self) -> Result<wgpu::SwapChainFrame, wgpu::SwapChainError> {
    self.swap_chain.get_current_frame()
  }
}
