use crate::*;

/// The render logic that not related to specific rendering "business" logic
pub struct RenderingRoot {
  render_memory: FunctionMemory,
  pool: AttachmentPool,
  pass_info_pool: PassInfoPool,
  frame_index: u64,
  stat_frame_time_in_ms: StatisticStore<f32>,
  last_render_timestamp: Option<Instant>,
  swap_chain: ApplicationWindowSurface,
  statistics: FramePassStatistics,
  enable_statistic_collect: bool,
  any_render_change: ChangeNotifier,
  gpu: GPU,
}

impl RenderingRoot {
  pub fn new(gpu: &GPU, swap_chain: ApplicationWindowSurface) -> Self {
    Self {
      render_memory: Default::default(),
      any_render_change: Default::default(),
      pass_info_pool: Default::default(),
      pool: init_attachment_pool(gpu),
      frame_index: 0,
      stat_frame_time_in_ms: StatisticStore::new(200),
      last_render_timestamp: Default::default(),
      swap_chain,
      enable_statistic_collect: false,
      statistics: FramePassStatistics::new(64, gpu),
      gpu: gpu.clone(),
    }
  }

  pub fn frame_index(&self) -> u64 {
    self.frame_index
  }

  pub fn notify_change(&self) {
    self.any_render_change.do_wake();
  }

  pub fn is_hdr(&self) -> bool {
    self
      .swap_chain
      .internal(|surface| surface.config.format == TextureFormat::Rgba16Float)
  }

  pub fn cleanup(&mut self, share_cx: &mut SharedHooksCtx) {
    let mut dcx = QueryGPUHookDropCx { share_cx };

    self.render_memory.cleanup(&mut dcx as *mut _ as *mut ());
  }

  pub fn setup_init_config(&self, init_config: &mut ViewerInitConfig) {
    init_config.present_mode = self.swap_chain.internal(|v| v.config.present_mode);
  }

  fn init_frame(&mut self) {
    self.frame_index += 1;
    let now = Instant::now();
    if let Some(last_frame_time) = self.last_render_timestamp.take() {
      self.stat_frame_time_in_ms.insert(
        now.duration_since(last_frame_time).as_secs_f32() * 1000.,
        self.frame_index,
      );
    }
    self.last_render_timestamp = Some(now);
  }

  pub fn inspect(
    &mut self,
    shared_ctx: &mut SharedHooksCtx,
    inspector: &mut dyn Inspector,
    rendering: &mut Viewer3dRenderingCtx,
    viewports: &[ViewerViewPort],
  ) {
    if !self.render_memory.created {
      return;
    }

    let gpu = self.gpu.clone();
    shared_ctx.reset_visiting();
    QueryGPUHookCx {
      memory: &mut self.render_memory,
      gpu: &gpu,
      stage: GPUQueryHookStage::Inspect(inspector),
      shared_ctx,
      storage_allocator: rendering.storage_allocator(),
      waker: futures::task::waker(self.any_render_change.clone()),
    }
    .execute(|cx| rendering.use_viewer_scene_renderer(cx, viewports));
  }

  pub fn draw_canvas(
    &mut self,
    canvas: &RenderTargetView,
    task_spawner: &TaskSpawner,
    content: &Viewer3dContent,
    shared_ctx: &mut SharedHooksCtx,
    rendering: &mut Viewer3dRenderingCtx,
  ) {
    self.init_frame();

    let gpu = self.gpu.clone();
    let mut immediate_results = Default::default();

    let statistics = self
      .enable_statistic_collect
      .then(|| self.statistics.create_resolver(self.frame_index));

    let mut ctx = FrameCtx::new(
      &gpu,
      canvas.size(),
      &self.pool,
      &self.pass_info_pool,
      statistics,
    );

    let upstream = futures::task::noop_waker();
    let any_changed = self.any_render_change.update(&upstream);
    let requested_render_views = rendering.check_should_render_and_copy_cached(
      canvas,
      &content.viewports,
      &mut ctx,
      any_changed,
    );

    if !requested_render_views.is_empty() {
      let mut pool = AsyncTaskPool::default();

      {
        let _ = trace_span!("spawn tasks to maintain renderer").entered();
        shared_ctx.reset_visiting();
        QueryGPUHookCx {
          memory: &mut self.render_memory,
          gpu: &gpu,
          waker: futures::task::waker(self.any_render_change.clone()),
          stage: GPUQueryHookStage::Update {
            spawner: task_spawner,
            task_pool: &mut pool,
            change_collector: &mut Default::default(),
            immediate_results: &mut immediate_results,
          },
          shared_ctx,
          storage_allocator: rendering.storage_allocator(),
        }
        .execute(|cx| rendering.use_viewer_scene_renderer(cx, &content.viewports));
      }

      let mut task_pool_result = {
        let _ = trace_span!("wait maintain renderer task finish").entered();
        pollster::block_on(pool.all_async_task_done())
      };

      let renderer = {
        let _ = trace_span!("maintain(gpu) and create renderer instance").entered();
        task_pool_result
          .token_based_result
          .extend(immediate_results.drain());
        shared_ctx.reset_visiting();

        QueryGPUHookCx {
          memory: &mut self.render_memory,
          gpu: &gpu,
          stage: GPUQueryHookStage::CreateRender {
            task: task_pool_result,
            encoder: &mut ctx.encoder,
          },
          shared_ctx,
          waker: futures::task::waker(self.any_render_change.clone()),
          storage_allocator: rendering.storage_allocator(),
        }
        .execute(|cx| {
          rendering
            .use_viewer_scene_renderer(cx, &content.viewports)
            .unwrap()
        })
      };

      let waker = futures::task::waker(self.any_render_change.clone());
      rendering.render(
        &requested_render_views,
        canvas,
        content,
        renderer,
        &mut ctx,
        &waker,
      );
    }

    drop(ctx);

    noop_ctx!(cx);
    self.statistics.poll(cx);
  }

  pub fn egui(
    &mut self,
    ui: &mut egui::Context,
    show_frame_info: &mut bool,
    last_frame_cpu_time: f32,
    frame_cpu_time_stat: &mut StatisticStore<f32>,
  ) {
    egui::Window::new("Frame Rendering Info")
      .open(show_frame_info)
      .vscroll(true)
      .show(ui, |ui| {
        let mut is_hdr = false;
        let mut ui = UiWithChangeInfo(ui, false);
        self.swap_chain.internal(|surface| {
          is_hdr = surface.config.format == TextureFormat::Rgba16Float;
          ui.collapsing("Swapchain config", |ui| {
            let cap = surface.capabilities();
            let default_none_hdr_format = get_default_preferred_format(cap);
            let support_hdr = cap.formats.contains(&TextureFormat::Rgba16Float);

            ui.add_enabled_ui(support_hdr, |ui| {
              ui.checkbox(&mut is_hdr, "enable hdr rendering")
                .on_disabled_hover_text("current platform does not support hdr rendering");
              if is_hdr {
                surface.config.format = TextureFormat::Rgba16Float;
              } else {
                surface.config.format = default_none_hdr_format;
              }
            });

            let current = surface.config.present_mode;

            let cap = surface.capabilities().present_modes.clone();

            let present_mode_ui = |ui: &mut UiWithChangeInfo,
                                   mode: PresentMode,
                                   name: &str,
                                   config: &mut PresentMode| {
              let supported = cap.contains(&mode)
                || mode == PresentMode::AutoVsync
                || mode == PresentMode::AutoNoVsync;
              ui.add_enabled_ui(supported, |ui| {
                ui.selectable_value(config, mode, name)
                  .on_disabled_hover_text("not supported");
              })
            };

            egui::ComboBox::from_label("present mode")
              .selected_text(format!("{:?}", current))
              .show_ui_changed(ui, |ui| {
                let target = &mut surface.config.present_mode;
                present_mode_ui(ui, PresentMode::AutoVsync, "AutoVsync", target);
                present_mode_ui(ui, PresentMode::AutoNoVsync, "AutoNoVsync", target);
                present_mode_ui(ui, PresentMode::Fifo, "Fifo", target);
                present_mode_ui(ui, PresentMode::FifoRelaxed, "FifoRelaxed", target);
                present_mode_ui(ui, PresentMode::Immediate, "Immediate", target);
                present_mode_ui(ui, PresentMode::Mailbox, "Mailbox", target);
              });
          });
        });

        ui.separator();

        if ui.1 {
          self.notify_change();
        }

        let average_frame_cpu_time = frame_cpu_time_stat.history_average();

        time_graph(
          &mut ui,
          &self.stat_frame_time_in_ms,
          last_frame_cpu_time,
          average_frame_cpu_time,
        );

        ui.label("frame pass pipeline statistics:");
        ui.separator();

        ui.checkbox(
          &mut self.enable_statistic_collect,
          "enable_statistic_collect",
        );

        if self.enable_statistic_collect {
          if self.statistics.collected.is_empty() {
            ui.label("no statistics info available");
          } else {
            if !self.statistics.pipeline_query_supported {
              ui.label("note: pipeline query not supported on this platform");
            } else {
              let statistics = &mut self.statistics;
              ui.collapsing("pipeline_info", |ui| {
                statistics.collected.iter().for_each(|(name, info)| {
                  if let Some((value, index)) = &info.pipeline.get_latest() {
                    #[allow(dead_code)]
                    #[derive(Debug)] // just to impl Debug
                    struct DeviceDrawStatistics2 {
                      pub vertex_shader_invocations: u64,
                      pub clipper_invocations: u64,
                      pub clipper_primitives_out: u64,
                      pub fragment_shader_invocations: u64,
                      pub compute_shader_invocations: u64,
                    }

                    impl From<DeviceDrawStatistics> for DeviceDrawStatistics2 {
                      fn from(value: DeviceDrawStatistics) -> Self {
                        Self {
                          vertex_shader_invocations: value.vertex_shader_invocations,
                          clipper_invocations: value.clipper_invocations,
                          clipper_primitives_out: value.clipper_primitives_out,
                          fragment_shader_invocations: value.fragment_shader_invocations,
                          compute_shader_invocations: value.compute_shader_invocations,
                        }
                      }
                    }

                    ui.collapsing(name, |ui| {
                      ui.label(format!("frame index: {:?}", index));
                      ui.label(format!("{:#?}", DeviceDrawStatistics2::from(*value)));
                    });
                  }
                });
              });
            }
            if !self.statistics.time_query_supported {
              ui.label("warning: time query not supported");
            } else {
              let statistics = &mut self.statistics;
              ui.collapsing("time_info", |ui| {
                statistics.collected.iter().for_each(|(name, info)| {
                  if let Some((value, _)) = &info.time.get_latest() {
                    let name = format!("{}: {:.2}ms", name, value);
                    ui.label(name);
                  }
                });
              });
            }

            if ui.button("clear").clicked() {
              self.statistics.clear_history(self.statistics.max_history);
            }
          }
        }
      });
  }
}

fn time_graph(
  ui: &mut UiWithChangeInfo,
  stat_frame_time_in_ms: &StatisticStore<f32>,
  last_frame_cpu_time: f32,
  average_frame_cpu_time: f32,
) {
  ui.collapsing("time graph", |ui| {
    let ui = &mut ui.0;
    ui.label(format!(
      "last frame cpu time: {:.2} ms",
      last_frame_cpu_time
    ));

    ui.label(format!(
      "average cpu time: {:.2} ms",
      average_frame_cpu_time
    ));
    if let Some((t, _)) = stat_frame_time_in_ms.get_latest() {
      ui.label(format!(
        "last frame time: {:.2} ms, fps: {:.2}",
        t,
        1000. / t
      ));
    }
    let t = stat_frame_time_in_ms.history_average();
    ui.label(format!(
      "average frame time: {:.2} ms, fps: {:.2}",
      t,
      1000. / t
    ));
    if let Some(times) = stat_frame_time_in_ms.iter_history_from_oldest_latest() {
      let graph_height = 200.;
      let graph_width = 300.;
      let (res, painter) = ui.allocate_painter(
        egui::Vec2 {
          x: graph_width,
          y: graph_height,
        },
        egui::Sense::empty(),
      );
      let x_start = res.rect.left();
      let y_start = res.rect.top();
      let x_step = graph_width / stat_frame_time_in_ms.history_size() as f32;

      let warning_time_threshold = 1000. / 60.;
      let serious_warning_time_threshold = 1000. / 15.;
      let max_time = stat_frame_time_in_ms
        .history_max()
        .copied()
        .unwrap_or(warning_time_threshold);
      for (idx, t) in times.enumerate() {
        if let Some(&t) = t {
          let height = t / max_time * graph_height;
          let color = if t >= serious_warning_time_threshold {
            egui::Color32::RED
          } else if t >= warning_time_threshold {
            egui::Color32::ORANGE
          } else if ui.visuals().dark_mode {
            egui::Color32::WHITE
          } else {
            egui::Color32::BLACK
          };
          painter.rect_filled(
            egui::Rect {
              min: egui::pos2(
                x_start + idx as f32 * x_step,
                y_start + (graph_height - height),
              ),
              max: egui::pos2(x_start + (idx + 1) as f32 * x_step, y_start + graph_height),
            },
            0.,
            color,
          );
        }
      }
    } else {
      ui.label("frame time graph not available");
    }
  });
}
