use std::rc::Rc;

pub mod default_scene;
pub use default_scene::*;
pub mod pipeline;
pub use pipeline::*;

pub mod controller;
pub use controller::*;
use rendiation_algebra::Mat4;
use rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig;
pub mod selection;

pub mod helpers;
use self::{
  helpers::{axis::AxisHelper, camera::CameraHelpers, grid::GridHelper, ground::GridGround},
  selection::SelectionSet,
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
  fn draw_canvas(&mut self, gpu: &Rc<GPU>, canvas: GPU2DTextureView) {
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
    self.terminal.check_execute(&mut self.content);
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
  pub(crate) content: Viewer3dContent,
  ctx: Option<Viewer3dRenderingCtx>,
  pub(crate) terminal: Terminal,
  size: Size,
}

impl Default for ViewerImpl {
  fn default() -> Self {
    let mut viewer = Self {
      content: Viewer3dContent::new(),
      size: Size::from_u32_pair_min_one((100, 100)),
      terminal: Default::default(),
      ctx: None,
    };

    viewer
      .terminal
      .register_command("load-gltf", |viewer, _parameters| {
        // let path = parameters.first().unwrap();

        rendiation_scene_gltf_loader::load_gltf_test(
          // path,
          "C:/Users/mk/Desktop/develop/glTF-Sample-Models/2.0/Suzanne/glTF/Suzanne.gltf",
          // "C:/Users/mk/Desktop/develop/glTF-Sample-Models/2.0/Sponza/glTF/Sponza.gltf",
          // "/Users/mikialex/dev/glTF-Sample-Models/2.0/Box/glTF/Box.gltf",
          &mut viewer.scene,
        )
        .unwrap();
      });

    viewer
  }
}

pub struct Viewer3dContent {
  pub scene: Scene<WebGPUScene>,
  pub ground: GridGround,
  pub pick_config: MeshBufferIntersectConfig,
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

  pub fn render(&mut self, target: RenderTargetView, content: &mut Viewer3dContent) {
    content.scene.maintain();
    self.resources.maintain();

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool, &mut self.resources);

    self.pipeline.render(&mut ctx, content, target);

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
      ground: Default::default(),
      controller,
      pick_config: Default::default(),
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

    let normalized_screen_position = position_info
      .compute_normalized_position_in_canvas_coordinate(states)
      .into();

    // todo, get correct size from render ctx side
    let camera_view_size = Size::from_usize_pair_min_one((
      position_info.size.width as usize,
      position_info.size.height as usize,
    ));

    let interactive_ctx = self.scene.build_interactive_ctx(
      normalized_screen_position,
      camera_view_size,
      &self.pick_config,
    );

    let mut ctx = EventCtx3D::new(states, event, &position_info, &self.scene, &interactive_ctx);

    let keep_target_for_gizmo = self.gizmo.event(&mut ctx);

    if !self.gizmo.has_active() {
      self.controller.event(event, bound);
    }

    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event) {
      if let Some((nearest, _)) = self.scene.interaction_picking(&interactive_ctx) {
        self.selections.clear();

        self
          .selections
          .select(SceneModelShareable::as_renderable(nearest));

        self.gizmo.set_target(nearest.get_node().into());
      } else if !keep_target_for_gizmo {
        self.gizmo.set_target(None);
      }
    }
  }

  pub fn update_state(&mut self) {
    if let Some(camera) = &mut self.scene.active_camera {
      camera.mutate(|camera| {
        self.controller.update(&mut ControlleeWrapper {
          controllee: &camera.node,
        });
      })
    }
  }
}

impl Default for Viewer3dContent {
  fn default() -> Self {
    Self::new()
  }
}

struct ControlleeWrapper<'a> {
  controllee: &'a SceneNode,
}

impl<'a> Transformed3DControllee for ControlleeWrapper<'a> {
  fn get_matrix(&self) -> Mat4<f32> {
    self.controllee.get_local_matrix()
  }

  fn set_matrix(&mut self, m: Mat4<f32>) {
    self.controllee.set_local_matrix(m)
  }
}
