pub mod examples;
use std::rc::Rc;

pub use examples::*;

pub mod view;
pub use view::*;

pub mod default_scene;
pub use default_scene::*;

use interphaser::*;
use rendiation_controller::{ControllerWinitAdapter, OrbitController};
use rendiation_webgpu::GPU;
use winit::event::{Event, WindowEvent};

use crate::*;

pub struct Viewer {
  _counter: Counter,
  todo: Todo,
  viewer: ViewerInner,
}

impl Viewer {
  pub fn new() -> Self {
    let todo = Todo {
      items: vec![
        TodoItem {
          name: String::from("t1中文测试"),
          id: 0,
        },
        TodoItem {
          name: String::from("test 2"),
          id: 1,
        },
        TodoItem {
          name: String::from("test gh3"),
          id: 2,
        },
      ],
    };
    Viewer {
      _counter: Counter { count: 0 },
      todo,
      viewer: ViewerInner {
        content: Viewer3dContent::new(),
        size: (0, 0),
        ctx: None,
      },
    }
  }
}

pub fn perf_panel() -> impl UIComponent<Viewer> {
  Text::default()
    .with_line_wrap(LineWrap::Multiple)
    .with_horizon_align(HorizontalAlign::Left)
    .with_vertical_align(VerticalAlign::Top)
    .bind_with_ctx(|s, _t, ctx| {
      let content = format!(
        "frame_id: {}\nupdate_time: {}\nlayout_time: {}\nrendering_prepare_time: {}\nrendering_dispatch_time: {}",
        ctx.last_frame_perf_info.frame_id,
        ctx.last_frame_perf_info.update_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.layout_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.rendering_prepare_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.rendering_dispatch_time.as_micros() as f32 / 1000.,
      );
      s.content.set(content);
    })
    .extend(Container::size((500., 200.)))
}

pub fn create_ui() -> impl UIComponent<Viewer> {
  absolute_group()
    .child(AbsChild::new(
      GPUCanvas::default().lens(lens!(Viewer, viewer)),
    ))
    .child(AbsChild::new(build_todo().lens(lens!(Viewer, todo))))
    .child(AbsChild::new(perf_panel()))
    .extend(AbsoluteAnchor::default())
}

impl CanvasPrinter for ViewerInner {
  fn draw_canvas(&mut self, gpu: &GPU, canvas: Rc<wgpu::TextureView>) {
    self.content.update_state();
    self
      .ctx
      .get_or_insert_with(|| Default::default())
      .render(canvas, gpu, &mut self.content)
  }

  fn event(&mut self, event: &winit::event::Event<()>) {
    self.content.event(event)
  }

  fn update_render_size(&mut self, layout_size: (f32, f32), gpu: &GPU) -> (u32, u32) {
    let new_size = (layout_size.0 as u32, layout_size.1 as u32);
    if let Some(ctx) = &mut self.ctx {
      if self.size != new_size {
        ctx.resize_view(gpu, new_size)
      }
    }
    self.size = new_size;
    new_size
  }
}

pub struct ViewerInner {
  content: Viewer3dContent,
  size: (u32, u32),
  ctx: Option<Viewer3dRenderingCtx>,
}

pub struct Viewer3dContent {
  scene: Scene,
  controller: ControllerWinitAdapter<OrbitController>,
}

#[derive(Default)]
pub struct Viewer3dRenderingCtx {
  pipeline: SimplePipeline,
}

impl Viewer3dRenderingCtx {
  pub fn resize_view(&mut self, gpu: &GPU, size: (u32, u32)) {
    // self.forward.resize(gpu, size)
    todo!()
  }

  pub fn render(&mut self, target: Rc<wgpu::TextureView>, gpu: &GPU, scene: &mut Viewer3dContent) {
    scene.scene.maintain(&gpu.device, &gpu.queue);

    todo!()
    // gpu.render_pass(
    //   &mut RenderPassDispatcher {
    //     scene: &mut scene.scene,
    //     pass: &mut self.forward,
    //   },
    //   target.as_ref(),
    // );
  }
}

impl Viewer3dContent {
  pub fn new() -> Self {
    let mut scene = Scene::new();

    load_default_scene(&mut scene);

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    let app = Self { scene, controller };
    app
  }

  pub fn resize_view(&mut self, size: (f32, f32)) {
    if let Some(camera) = &mut self.scene.active_camera {
      camera.projection.resize(size)
    }
  }

  pub fn event(&mut self, event: &Event<()>) {
    self.controller.event(event);

    if let Event::WindowEvent { event, .. } = event {
      if let WindowEvent::Resized(size) = event {
        self.resize_view((size.width as f32, size.height as f32));
      }
    }
  }

  pub fn update_state(&mut self) {
    if let Some(camera) = &mut self.scene.active_camera {
      let node = self.scene.nodes.get_node_mut(camera.node).data_mut();
      self.controller.update(node);
    }
  }
}
