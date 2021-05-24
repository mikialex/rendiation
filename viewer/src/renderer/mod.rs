use arena::Arena;

use self::swap_chain::SwapChain;
mod swap_chain;

pub trait Renderable {
  fn render<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
  );
}

pub struct Renderer {
  instance: wgpu::Instance,
  adaptor: wgpu::Adapter,
  pub(crate) device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: SwapChain,

  // pipeline_cache: Vec<wgpu::RenderPipeline>,
  // bindgroup_cache: Vec<wgpu::BindGroup>,
  pub(crate) buffers: Arena<wgpu::Buffer>,
}

impl Renderer {
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
      (size.width as usize, size.height as usize),
    );

    Self {
      // pipeline_cache: Vec::new(),
      buffers: Arena::new(),
      instance,
      adaptor,
      device,
      queue,
      swap_chain,
    }
  }
  pub fn render(&mut self, pass_des: &wgpu::RenderPassDescriptor, renderable: &mut dyn Renderable) {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
      let mut pass = encoder.begin_render_pass(pass_des);
      renderable.render(self, &mut pass, pass_des);
    }

    self.queue.submit(Some(encoder.finish()));
  }
  pub fn resize(&mut self, size: (usize, usize)) {
    self.swap_chain.resize(size, &self.device);
  }

  pub(crate) fn create_shader_flags(&self) -> wgpu::ShaderFlags {
    let mut flags = wgpu::ShaderFlags::VALIDATION;
    match self.adaptor.get_info().backend {
      wgpu::Backend::Metal | wgpu::Backend::Vulkan => {
        flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
      }
      _ => (), //TODO
    }
    flags
  }
  pub(crate) fn get_prefer_target_format(&self) -> wgpu::TextureFormat {
    self.swap_chain.swap_chain_descriptor.format
  }

  pub fn get_current_frame(&mut self) -> Result<wgpu::SwapChainFrame, wgpu::SwapChainError> {
    self.swap_chain.get_current_frame()
  }
}
