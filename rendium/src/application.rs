use rendiation_webgpu::renderer::SwapChain;
use rendiation_webgpu::WGPURenderer;
use winit::event::WindowEvent;

pub struct AppRenderCtx<'a> {
  pub renderer: &'a mut WGPURenderer,
  pub swap_chain: &'a mut SwapChain,
}

pub trait Application: 'static + Sized {
  fn init(renderer: &mut WGPURenderer, swap_chain: &SwapChain) -> Self;
  fn update(&mut self, event: &winit::event::Event<()>, renderer: &mut AppRenderCtx);
}

pub fn run<E: Application>(title: &str) {
  futures::executor::block_on(run_async::<E>(title));
}

pub async fn run_async<E: Application>(title: &str) {
  use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
  };

  let event_loop = EventLoop::new();
  log::info!("Initializing the window...");

  #[cfg(not(feature = "gl"))]
  let (window, size, surface) = {
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title(title);
    // #[cfg(windows_OFF)] //TODO
    // {
    //     use winit::platform::windows::WindowBuilderExtWindows;
    //     builder = builder.with_no_redirection_bitmap(true);
    // }
    let window = builder.build(&event_loop).unwrap();
    let size = window.inner_size();
    let surface = rendiation_webgpu::Surface::create(&window);
    (window, size, surface)
  };

  #[cfg(feature = "gl")]
  let (window, instance, size, surface) = {
    let wb = winit::WindowBuilder::new();
    let cb = wgpu::glutin::ContextBuilder::new().with_vsync(true);
    let context = cb.build_windowed(wb, &event_loop).unwrap();
    context.window().set_title(title);

    let hidpi_factor = context.window().hidpi_factor();
    let size = context
      .window()
      .get_inner_size()
      .unwrap()
      .to_physical(hidpi_factor);

    let (context, window) = unsafe { context.make_current().unwrap().split() };

    let instance = wgpu::Instance::new(context);
    let surface = instance.get_surface();

    (window, instance, size, surface)
  };

  let mut renderer = WGPURenderer::new(&surface).await;

  let mut swap_chain = SwapChain::new(
    surface,
    (size.width as usize, size.height as usize),
    &renderer,
  );

  log::info!("Initializing the application...");
  let mut example = E::init(&mut renderer, &swap_chain);

  log::info!("Entering render loop...");
  event_loop.run(move |event, _, control_flow| {
    match &event {
      event::Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => {
        log::info!("Resizing to {:?}", size);
        swap_chain.resize(
          (size.width as usize, size.height as usize),
          &renderer.device,
        );
      }
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::CloseRequested => {
          *control_flow = ControlFlow::Exit;
        }
        _ => {}
      },
      _ => (),
    }

    let mut ctx = AppRenderCtx {
      renderer: &mut renderer,
      swap_chain: &mut swap_chain,
    };

    example.update(&event, &mut ctx);
  });
}
