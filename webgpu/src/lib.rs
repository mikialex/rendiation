#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(const_fn_transmute)]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

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
pub use swap_chain::*;
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
  pub async fn new(surface_provider: &dyn SurfaceProvider) -> (Self, GPUSwapChain) {
    let backend = wgpu::BackendBit::PRIMARY;
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

    let swap_chain = GPUSwapChain::new(&adaptor, &device, surface, size);

    (
      Self {
        instance,
        adaptor,
        device,
        queue,
      },
      swap_chain,
    )
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
}
