use std::cell::RefCell;
use std::sync::Arc;

pub mod contents;
pub use contents::*;

pub mod default_scene;
pub use default_scene::*;
pub mod pipeline;
use futures::Future;
pub use pipeline::*;

pub mod controller;
pub use controller::*;
use reactive::EventSource;
use rendiation_algebra::Mat4;
use rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig;
use rendiation_scene_interaction::WebGPUScenePickingExt;
pub mod selection;

pub mod helpers;
use interphaser::winit::event::{ElementState, Event, MouseButton};
use interphaser::*;
use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};
use rendiation_texture::Size;
use webgpu::*;

use self::{
  helpers::{axis::AxisHelper, camera::CameraHelpers, grid::GridHelper, ground::GridGround},
  selection::SelectionSet,
};
use crate::*;

impl CanvasPrinter for ViewerImpl {
  fn draw_canvas(&mut self, gpu: &Arc<GPU>, canvas: GPU2DTextureView) {
    self.content.update_state();
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
    let mut ctx = CommandCtx {
      scene: &self.content.scene,
      rendering: self.ctx.as_mut(),
    };

    self.terminal.check_execute(&mut ctx);
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
  pub io_executor: futures::executor::ThreadPool,
  pub compute_executor: rayon::ThreadPool,
}

impl Default for ViewerImpl {
  fn default() -> Self {
    let io_executor = futures::executor::ThreadPool::builder()
      .name_prefix("rendiation_io_threads")
      .pool_size(2)
      .create()
      .unwrap();

    let compute_executor = rayon::ThreadPoolBuilder::new()
      .thread_name(|i| format!("rendiation_compute_threads-{i}"))
      .build()
      .unwrap();

    let mut viewer = Self {
      content: Viewer3dContent::new(),
      size: Size::from_u32_pair_min_one((100, 100)),
      terminal: Default::default(),
      ctx: None,
      io_executor,
      compute_executor,
    };

    register_default_commands(&mut viewer.terminal);

    viewer
  }
}

pub struct Viewer3dContent {
  pub scene: Scene,
  pub scene_derived: SceneNodeDeriveSystem,
  pub scene_bounding: SceneModelWorldBoundingSystem,
  pub pick_config: MeshBufferIntersectConfig,
  pub selections: SelectionSet,
  pub controller: ControllerWinitAdapter<OrbitController>,
  // refcell is to support updating when rendering, have to do this, will be remove in future
  pub widgets: RefCell<WidgetContent>,
}

pub struct WidgetContent {
  pub ground: GridGround,
  pub axis_helper: AxisHelper,
  pub grid_helper: GridHelper,
  pub camera_helpers: CameraHelpers,
  pub gizmo: Gizmo,
}

pub struct Viewer3dRenderingCtx {
  pipeline: ViewerPipeline,
  pool: ResourcePool,
  resources: GlobalGPUSystem,
  gpu: Arc<GPU>,
  on_encoding_finished: EventSource<ViewRenderedState>,
}

#[derive(Clone)]
struct ViewRenderedState {
  target: RenderTargetView,
  device: GPUDevice,
  queue: GPUQueue,
}

#[derive(Debug)]
pub enum ViewerRenderResultReadBackErr {
  GPU(webgpu::BufferAsyncError),
  UnableToReadSurfaceTexture,
}

impl ViewRenderedState {
  async fn read(self) -> Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr> {
    match self.target {
      RenderTargetView::Texture(tex) => {
        // I have to write this, because I don't know why compiler can't known the encoder is
        // dropped and will not across the await point
        let buffer = {
          let mut encoder = self.device.create_encoder();

          let buffer = encoder.read_texture_2d(
            &self.device,
            &tex.resource.clone().try_into().unwrap(),
            ReadRange {
              size: Size::from_u32_pair_min_one((
                tex.resource.desc.size.width,
                tex.resource.desc.size.height,
              )),
              offset_x: 0,
              offset_y: 0,
            },
          );
          self.queue.submit(Some(encoder.finish()));
          buffer
        };

        buffer.await.map_err(ViewerRenderResultReadBackErr::GPU)
      }
      RenderTargetView::SurfaceTexture { .. } => {
        // note: maybe surface could supported by extra copy, but I'm not sure the surface texture's
        // usage flag.
        Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture)
      }
    }
  }
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: Arc<GPU>) -> Self {
    let gpu_resources = GlobalGPUSystem::new(&gpu);
    Self {
      pipeline: ViewerPipeline::new(gpu.as_ref()),
      gpu,
      resources: gpu_resources,
      pool: Default::default(),
      on_encoding_finished: Default::default(),
    }
  }

  /// only texture could be read. caller must sure the target passed in render call not using
  /// surface.
  pub fn read_next_render_result(
    &mut self,
  ) -> impl Future<Output = Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr>> {
    use futures::FutureExt;
    self
      .on_encoding_finished
      .once_future(|result| result.clone().read())
      .flatten()
  }

  pub fn resize_view(&mut self) {
    self.pool.clear();
  }

  pub fn render(&mut self, target: RenderTargetView, content: &mut Viewer3dContent) {
    content.maintain();
    self.resources.maintain();

    let (scene_resource, content_res) = self
      .resources
      .get_or_create_scene_sys_with_content(&content.scene, &content.scene_derived);
    let resource = content_res.read().unwrap();

    let scene = content.scene.read();

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool);
    let scene_res = SceneRenderResourceGroup {
      scene: &scene,
      resources: &resource,
      scene_resources: scene_resource,
      node_derives: &content.scene_derived,
    };

    self.pipeline.render(&mut ctx, content, &target, &scene_res);
    ctx.final_submit();

    self.on_encoding_finished.emit(&ViewRenderedState {
      target,
      device: self.gpu.device.clone(),
      queue: self.gpu.queue.clone(),
    })
  }
}

impl Viewer3dContent {
  pub fn new() -> Self {
    let (scene, scene_derived) = SceneInner::new();

    let scene_bounding = SceneModelWorldBoundingSystem::new(&scene, &scene_derived);

    load_default_scene(&scene);

    let s = scene.clone();
    let inner = s.read();

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    let axis_helper = AxisHelper::new(inner.root());
    let grid_helper = GridHelper::new(inner.root(), Default::default());

    let gizmo = Gizmo::new(inner.root(), &scene_derived);

    let widgets = WidgetContent {
      ground: Default::default(),
      axis_helper,
      grid_helper,
      camera_helpers: Default::default(),
      gizmo,
    };

    Self {
      scene,
      scene_derived,
      scene_bounding,
      controller,
      pick_config: Default::default(),
      selections: Default::default(),
      widgets: RefCell::new(widgets),
    }
  }

  pub fn maintain(&mut self) {
    self.scene_derived.maintain();
    self.scene_bounding.maintain();
  }

  pub fn resize_view(&mut self, size: (f32, f32)) {
    if let Some(camera) = &self.scene.read().active_camera {
      camera.resize(size)
    }
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    self.maintain();
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

    let scene = &self.scene.read();

    let interactive_ctx = scene.build_interactive_ctx(
      normalized_screen_position,
      camera_view_size,
      &self.pick_config,
      &self.scene_derived,
    );

    let mut ctx = EventCtx3D::new(
      states,
      event,
      &position_info,
      scene,
      &interactive_ctx,
      &self.scene_derived,
    );

    let widgets = self.widgets.get_mut();
    let gizmo = &mut widgets.gizmo;

    let keep_target_for_gizmo = gizmo.event(&mut ctx);

    if !gizmo.has_active() {
      self.controller.event(event, bound);
    }

    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event) {
      if let Some((nearest, _)) =
        scene.interaction_picking(&interactive_ctx, &mut self.scene_bounding)
      {
        self.selections.clear();
        self.selections.select(nearest);

        gizmo.set_target(nearest.get_node().into(), &self.scene_derived);
      } else if !keep_target_for_gizmo {
        gizmo.set_target(None, &self.scene_derived);
      }
    }
  }

  pub fn update_state(&mut self) {
    self.maintain();

    let widgets = self.widgets.get_mut();
    let gizmo = &mut widgets.gizmo;
    gizmo.update(&self.scene_derived);
    if let Some(camera) = &self.scene.read().active_camera {
      self.controller.update(&mut ControlleeWrapper {
        controllee: &camera.read().node,
      });
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
