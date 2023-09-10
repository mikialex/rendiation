use std::cell::RefCell;

use interphaser::{
  mouse,
  winit::event::{ElementState, Event, MouseButton},
  CanvasWindowPositionInfo, WindowState,
};
use reactive::PollUtils;
use rendiation_algebra::Mat4;
use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};
use rendiation_mesh_core::MeshBufferIntersectConfig;
use rendiation_scene_interaction::WebGPUScenePickingExt;

use crate::*;

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

impl Viewer3dContent {
  pub fn new() -> Self {
    let (scene, scene_derived) = SceneImpl::new();

    let scene_core = scene.get_scene_core();

    let scene_bounding = SceneModelWorldBoundingSystem::new(&scene_core, &scene_derived);

    load_default_scene(&scene);

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    let axis_helper = AxisHelper::new(&scene.root());
    let grid_helper = GridHelper::new(&scene.root(), Default::default());

    let gizmo = Gizmo::new(&scene.root(), &scene_derived);

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
    let waker = futures::task::noop_waker_ref();
    let mut cx = std::task::Context::from_waker(waker);
    let _ = self
      .scene_derived
      .poll_until_pending_or_terminate_not_care_result(&mut cx);
    let _ = self
      .scene_bounding
      .poll_until_pending_or_terminate_not_care_result(&mut cx);
  }

  pub fn resize_view(&mut self, size: (f32, f32)) {
    if let Some(camera) = &self.scene.read().core.read().active_camera {
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

    let s = self.scene.read();
    let scene = &s.core.read();

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
    if let Some(camera) = &self.scene.read().core.read().active_camera {
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

pub struct WidgetContent {
  pub ground: GridGround,
  pub axis_helper: AxisHelper,
  pub grid_helper: GridHelper,
  pub camera_helpers: CameraHelpers,
  pub gizmo: Gizmo,
}
