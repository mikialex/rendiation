use std::{ops::DerefMut, rc::Rc};

pub mod default_scene;
pub use default_scene::*;
pub mod pipeline;
pub use pipeline::*;

pub mod controller;
pub use controller::*;
pub mod selection;

pub mod helpers;
use self::{
  helpers::{axis::AxisHelper, camera::CameraHelpers, grid::GridHelper},
  selection::{Picker, SelectionSet},
};

use interphaser::winit::event::{ElementState, Event, MouseButton};
use interphaser::*;
use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};
use rendiation_texture::Size;
use webgpu::*;

use crate::*;

impl CanvasPrinter for ViewerImpl {
  fn draw_canvas(&mut self, gpu: &Rc<GPU>, canvas: GPUTexture2dView) {
    self.content.update_state();
    self.content.gizmo.update();
    self
      .ctx
      .get_or_insert_with(|| Viewer3dRenderingCtx::new(gpu.clone()))
      .render(RenderTargetView::Texture(canvas), &mut self.content)
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
  ctx: Option<Viewer3dRenderingCtx>,
  size: Size,
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
  pub scene: Scene<WebGPUScene>,
  pub picker: Picker,
  pub selections: SelectionSet,
  pub controller: ControllerWinitAdapter<OrbitController>,
  pub axis_helper: AxisHelper,
  pub grid_helper: GridHelper,
  pub camera_helpers: CameraHelpers,
  pub gizmo: Gizmo,
}

pub struct Viewer3dRenderingCtx {
  pipeline: ViewerPipeline,
  pool: ResourcePool,
  resources: GPUResourceCache,
  gpu: Rc<GPU>,
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: Rc<GPU>) -> Self {
    Self {
      pipeline: ViewerPipeline::new(gpu.as_ref()),
      gpu,
      resources: Default::default(),
      pool: Default::default(),
    }
  }

  pub fn resize_view(&mut self) {
    self.pool.clear();
  }

  pub fn render(&mut self, target: RenderTargetView, scene: &mut Viewer3dContent) {
    scene.scene.maintain();

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool, &mut self.resources);

    self.pipeline.render(&mut ctx, scene, target);

    ctx.submit()
  }
}

impl Viewer3dContent {
  pub fn new() -> Self {
    let mut scene = Scene::new();

    load_default_scene(&mut scene);

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    let axis_helper = AxisHelper::new(scene.root());
    let grid_helper = GridHelper::new(scene.root(), Default::default());

    let gizmo = Gizmo::new(scene.root());

    Self {
      scene,
      controller,
      picker: Default::default(),
      selections: Default::default(),
      axis_helper,
      grid_helper,
      camera_helpers: Default::default(),
      gizmo,
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
    let bound = InputBound {
      origin: (
        position_info.absolute_position.x,
        position_info.absolute_position.y,
      )
        .into(),
      size: (position_info.size.width, position_info.size.height).into(),
    };

    self.gizmo.event(event, &position_info, states, &self.scene);
    self.controller.event(event, bound);

    #[allow(clippy::single_match)]
    match event {
      Event::WindowEvent { event, .. } => match event {
        winit::event::WindowEvent::MouseInput { state, button, .. } => {
          if *button == MouseButton::Left && *state == ElementState::Pressed {
            self.picker.pick_new(
              &self.scene,
              &mut self.selections,
              position_info
                .compute_normalized_position_in_canvas_coordinate(states)
                .into(),
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
        self
          .controller
          .update(node.deref_mut() as &mut dyn Transformed3DControllee);
      });
      camera.trigger_change()
    }
  }
}

impl Default for Viewer3dContent {
  fn default() -> Self {
    Self::new()
  }
}
