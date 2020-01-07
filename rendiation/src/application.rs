use crate::renderer::Renderer;
use winit::event::WindowEvent;
use crate::renderer::*;

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::mem::size_of;
    use std::slice::from_raw_parts;

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

pub trait Application<R: Renderer>: 'static + Sized {
    fn init(
        renderer: &WGPURenderer<R>
    ) -> (Self, Option<wgpu::CommandBuffer>);
    fn resize(
        &mut self,
        renderer: &WGPURenderer<R>
    ) -> Option<wgpu::CommandBuffer>;
    fn update(&mut self, event: WindowEvent);
    fn render(
        &mut self,
        frame: &wgpu::SwapChainOutput,
        device: &wgpu::Device,
        renderer: &mut R,
    ) -> wgpu::CommandBuffer;
}

pub fn run<R: Renderer, E: Application<R>>(title: &str) {
    use winit::{
        event,
        event_loop::{ControlFlow, EventLoop},
    };

    let event_loop = EventLoop::new();
    log::info!("Initializing the window...");

    #[cfg(not(feature = "gl"))]
    let (_window, hidpi_factor, size, surface) = {
        let window = winit::window::Window::new(&event_loop).unwrap();
        window.set_title(title);
        let hidpi_factor = window.hidpi_factor();
        let size = window.inner_size().to_physical(hidpi_factor);
        let surface = wgpu::Surface::create(&window);
        (window, hidpi_factor, size, surface)
    };

    #[cfg(feature = "gl")]
    let (_window, instance, hidpi_factor, size, surface) = {
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

        (window, instance, hidpi_factor, size, surface)
    };

    let mut renderer = WGPURenderer::new(surface, (size.width.round() as usize, size.height.round() as usize));

    log::info!("Initializing the example...");
    let (mut example, init_command_buf) = E::init(&renderer);
    if let Some(command_buf) = init_command_buf {
        renderer.queue.submit(&[command_buf]);
    }

    log::info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let physical = size.to_physical(hidpi_factor);
                log::info!("Resizing to {:?}", physical);
                renderer.resize(physical.width.round() as usize, physical.height.round() as usize);
                let command_buf = example.resize(&renderer);
                if let Some(command_buf) = command_buf {
                    renderer.queue.submit(&[command_buf]);
                }
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {
                    example.update(event);
                }
            },
            event::Event::EventsCleared => {
                let frame = renderer.swap_chain.get_next_texture();
                let command_buf = example.render(&frame, &renderer.device, &mut renderer.renderer);
                renderer.queue.submit(&[command_buf]);
            }
            _ => (),
        }
    });
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}
