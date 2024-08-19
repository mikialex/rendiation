use egui::epaint::Shadow;
use egui::Visuals;
use winit::window::Window;

use crate::*;

pub struct EguiContext<T> {
  inner: T,
  context: egui::Context,
  state: Option<egui_winit::State>,
  renderer: Option<egui_wgpu::Renderer>,
}

impl<T: Widget> Widget for EguiContext<T> {
  fn update_state(&mut self, cx: &mut DynCx) {
    {
      access_cx_mut!(cx, platform_event, PlatformEventInput);

      platform_event.window_state.mouse_position_in_ui = self.context.is_pointer_over_area();
    }
    access_cx!(cx, window, Window);
    access_cx!(cx, platform_event, PlatformEventInput);

    let state = self.state.get_or_insert_with(|| {
      let id = self.context.viewport_id();
      egui_winit::State::new(self.context.clone(), id, &window, None, None, None)
    });

    for event in &platform_event.accumulate_events {
      if let Event::WindowEvent { event, .. } = event {
        let _ = state.on_window_event(window, event);
      }
    }

    self.inner.update_state(cx);
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx!(cx, window, Window);
    self.begin_frame(window);

    cx.scoped_cx(&mut self.context, |cx| {
      self.inner.update_view(cx);
    });

    access_cx!(cx, window, Window);
    access_cx!(cx, gpu, GPU);
    access_cx!(cx, canvas, RenderTargetView);
    self.end_frame_and_draw(gpu, window, canvas);
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.inner.clean_up(cx)
  }
}

impl<T> EguiContext<T> {
  pub fn new(inner: T) -> EguiContext<T> {
    let egui_context = egui::Context::default();

    const BORDER_RADIUS: f32 = 2.0;

    let visuals = Visuals {
      window_rounding: egui::Rounding::same(BORDER_RADIUS),
      window_shadow: Shadow::NONE,
      ..Default::default()
    };

    egui_context.set_visuals(visuals);

    EguiContext {
      inner,
      context: egui_context,
      state: None,
      renderer: None,
    }
  }

  pub fn begin_frame(&mut self, window: &Window) {
    let state = self.state.as_mut().unwrap();
    self.context.begin_frame(state.take_egui_input(window));
  }

  pub fn end_frame_and_draw(&mut self, gpu: &GPU, window: &Window, target: &RenderTargetView) {
    let state = self.state.as_mut().unwrap();
    let view = target.as_view();

    let full_output = self.context.end_frame();

    state.handle_platform_output(window, full_output.platform_output);

    let tris = self
      .context
      .tessellate(full_output.shapes, full_output.pixels_per_point);

    // todo, recreate renderer if target spec changed
    let renderer = self.renderer.get_or_insert_with(|| {
      let output_color_format = target.format();
      let output_depth_format = None;
      let msaa_samples = 1;
      egui_wgpu::Renderer::new(
        &gpu.device,
        output_color_format,
        output_depth_format,
        msaa_samples,
        false,
      )
    });

    for (id, image_delta) in &full_output.textures_delta.set {
      renderer.update_texture(&gpu.device, &gpu.queue, *id, image_delta);
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

    renderer.update_buffers(
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
    renderer.render(&mut rpass, &tris, &screen_descriptor);
    drop(rpass);

    gpu.queue.submit(std::iter::once(encoder.finish()));

    for x in &full_output.textures_delta.free {
      renderer.free_texture(x)
    }
  }
}
