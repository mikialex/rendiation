use egui::epaint::Shadow;
use egui::Visuals;
use egui_wgpu::ScreenDescriptor;
use egui_winit::State;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::*;

pub struct EguiContext {
  context: egui::Context,
  state: egui_winit::State,
  renderer: egui_wgpu::Renderer,
}

impl EguiContext {
  pub fn new(
    device: &Device,
    output_color_format: TextureFormat,
    output_depth_format: Option<TextureFormat>,
    msaa_samples: u32,
    window: &Window,
  ) -> EguiContext {
    let egui_context = egui::Context::default();
    let id = egui_context.viewport_id();

    const BORDER_RADIUS: f32 = 2.0;

    let visuals = Visuals {
      window_rounding: egui::Rounding::same(BORDER_RADIUS),
      window_shadow: Shadow::NONE,
      ..Default::default()
    };

    egui_context.set_visuals(visuals);

    let egui_state = egui_winit::State::new(egui_context.clone(), id, &window, None, None);

    let egui_renderer = egui_wgpu::Renderer::new(
      device,
      output_color_format,
      output_depth_format,
      msaa_samples,
    );

    EguiContext {
      context: egui_context,
      state: egui_state,
      renderer: egui_renderer,
    }
  }

  pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) {
    let _ = self.state.on_window_event(window, event);
  }

  pub fn cx(&self) -> &egui::Context {
    &self.context
  }

  pub fn begin_frame(&mut self, window: &Window) {
    self.context.begin_frame(self.state.take_egui_input(window));
  }

  pub fn end_frame_and_draw(&mut self, gpu: &GPU, window: &Window, target: &RenderTargetView) {
    let view = target.as_view();

    let full_output = self.context.end_frame();

    self
      .state
      .handle_platform_output(window, full_output.platform_output);

    let tris = self
      .context
      .tessellate(full_output.shapes, full_output.pixels_per_point);

    for (id, image_delta) in &full_output.textures_delta.set {
      self
        .renderer
        .update_texture(&gpu.device, &gpu.queue, *id, image_delta);
    }

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
      size_in_pixels: [window.inner_size().width, window.inner_size().height],
      pixels_per_point: window.scale_factor() as f32,
    };

    let mut encoder =
      gpu
        .device
        .create_command_encoder(&rendiation_webgpu::CommandEncoderDescriptor {
          label: Some("GUI encoder"),
        });

    self.renderer.update_buffers(
      &gpu.device,
      &gpu.queue,
      &mut encoder,
      &tris,
      &screen_descriptor,
    );

    let mut rpass = encoder.begin_render_pass(&rendiation_webgpu::RenderPassDescriptor {
      color_attachments: &[Some(rendiation_webgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: rendiation_webgpu::Operations {
          load: rendiation_webgpu::LoadOp::Load,
          store: rendiation_webgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      label: Some("egui main render pass"),
      timestamp_writes: None,
      occlusion_query_set: None,
    });
    self.renderer.render(&mut rpass, &tris, &screen_descriptor);
    drop(rpass);

    gpu.queue.submit(std::iter::once(encoder.finish()));

    for x in &full_output.textures_delta.free {
      self.renderer.free_texture(x)
    }
  }
}
