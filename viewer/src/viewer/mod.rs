use std::rc::Rc;

pub mod default_scene;
pub use default_scene::*;

pub mod selection;

pub mod helpers;
use self::{
  helpers::axis::AxisHelper,
  selection::{Picker, SelectionSet},
};

use interphaser::*;
use rendiation_controller::{ControllerWinitAdapter, OrbitController};
use rendiation_texture::Size;
use rendiation_webgpu::GPU;
use winit::event::{ElementState, Event, MouseButton};

use crate::*;

impl CanvasPrinter for ViewerImpl {
  fn draw_canvas(&mut self, gpu: &Rc<GPU>, canvas: FrameTarget) {
    self.content.update_state();
    self
      .ctx
      .get_or_insert_with(|| Viewer3dRenderingCtx::new(gpu.clone()))
      .render(canvas, gpu, &mut self.content)
  }

  fn event(
    &mut self,
    event: &winit::event::Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    self.content.event(event, states, position_info)
  }

  fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size {
    let new_size = (layout_size.0 as u32, layout_size.1 as u32);
    let new_size = Size::from_u32_pair_min_one(new_size);
    if let Some(ctx) = &mut self.ctx {
      if self.size != new_size {
        ctx.resize_view();
        self.content.resize_view(layout_size);
      }
    }
    self.size = new_size;
    new_size
  }
}

pub struct ViewerImpl {
  content: Viewer3dContent,
  size: Size,
  ctx: Option<Viewer3dRenderingCtx>,
}

impl Default for ViewerImpl {
  fn default() -> Self {
    Self {
      content: Viewer3dContent::new(),
      size: Size::from_u32_pair_min_one((100, 100)),
      ctx: None,
    }
  }
}

pub struct Viewer3dContent {
  pub scene: Scene,
  pub picker: Picker,
  pub selections: SelectionSet,
  pub controller: ControllerWinitAdapter<OrbitController>,
  pub axis: AxisHelper,
}

pub struct Viewer3dRenderingCtx {
  pipeline: SimplePipeline,
  engine: RenderEngine,
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: Rc<GPU>) -> Self {
    Self {
      pipeline: SimplePipeline::new(gpu.as_ref()),
      engine: RenderEngine::new(gpu),
    }
  }

  pub fn resize_view(&mut self) {
    self.engine.notify_output_resized();
  }

  pub fn render(&mut self, target: FrameTarget, gpu: &GPU, scene: &mut Viewer3dContent) {
    scene.scene.maintain(gpu);

    self.engine.output = target.into();

    self.pipeline.render_simple(&self.engine, scene)
  }
}

impl Viewer3dContent {
  pub fn new() -> Self {
    let mut scene = Scene::new();

    load_default_scene(&mut scene);

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    let axis = AxisHelper::new(&scene.root);

    Self {
      scene,
      controller,
      picker: Default::default(),
      selections: Default::default(),
      axis,
    }
  }

  pub fn resize_view(&mut self, size: (f32, f32)) {
    if let Some(camera) = &mut self.scene.active_camera {
      camera.resize(size)
    }
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    self.controller.event(event);

    #[allow(clippy::single_match)]
    match event {
      Event::WindowEvent { event, .. } => match event {
        winit::event::WindowEvent::MouseInput { state, button, .. } => {
          let canvas_x = states.mouse_position.x - position_info.absolute_position.x;
          let canvas_y = states.mouse_position.y - position_info.absolute_position.y;

          if *button == MouseButton::Left && *state == ElementState::Pressed {
            let normalized_position = (
              canvas_x / position_info.size.width * 2. - 1.,
              -(canvas_y / position_info.size.height * 2. - 1.),
            );

            self.picker.pick_new(
              &self.scene,
              &mut self.selections,
              normalized_position.into(),
            );
          }
        }
        _ => {}
      },
      _ => {}
    }
  }

  pub fn update_state(&mut self) {
    if let Some(camera) = &mut self.scene.active_camera {
      camera.node.mutate(|node| {
        self.controller.update(node);
      });
    }
  }
}

impl Default for Viewer3dContent {
  fn default() -> Self {
    Self::new()
  }
}
