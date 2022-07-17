use std::{any::Any, cell::RefCell, rc::Rc};

use interphaser::{
  winit::event::{ElementState, Event, MouseButton, WindowEvent},
  CanvasWindowPositionInfo, WindowState,
};
use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::{
  mesh::MeshBufferHitPoint,
  tessellation::{CubeMeshParameter, IndexedMeshTessellator},
};

use crate::*;

pub struct Gizmo {
  // scale: AxisScaleGizmo,
  // rotation: RotationGizmo,
  translate: MovingGizmo,
}

impl Gizmo {
  pub fn new(root: &SceneNode) -> Self {
    let auto_scale = ViewAutoScalable {
      override_position: None,
      independent_scale_factor: 100.,
    };
    let auto_scale = Rc::new(RefCell::new(auto_scale));
    todo!()
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    states: &WindowState,
    scene: &Scene<WebGPUScene>,
  ) {
    self.translate.event(event, info, states, scene)
  }
}

pub struct AxisScaleGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  active: AxisActiveState,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_box() -> Box<dyn SceneRenderable> {
  let mesh = CubeMeshParameter::default().tessellate();
  let mesh = MeshCell::new(MeshSource::new(mesh));
  todo!();
}

pub struct RotationGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  active: AxisActiveState,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_rotation_circle() -> Box<dyn SceneRenderable> {
  let mut position = Vec::new();
  let segments = 50;
  for i in 0..segments {
    let p = i as f32 / segments as f32;
    position.push(Vec3::new(p.cos(), p.sin(), 0.))
  }
  todo!();
}

pub struct MovingGizmo {
  pub enabled: bool,
  states: MovingGizmoState,
  // x: Box<dyn SceneRenderableShareable>,
  // y: Box<dyn SceneRenderableShareable>,
  // z: Box<dyn SceneRenderableShareable>,
  // xy_hint: Box<dyn SceneRenderableShareable>,
  // xz_hint: Box<dyn SceneRenderableShareable>,
  // zy_hint: Box<dyn SceneRenderableShareable>,
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  view: Vec<Box<dyn SceneRenderableShareable>>,
}

fn build_axis_arrow(root: &SceneNode) -> Box<dyn SceneRenderableShareable> {
  todo!();
}

#[derive(Copy, Clone, Default)]
pub struct AxisActiveState {
  x: bool,
  y: bool,
  z: bool,
}

impl AxisActiveState {
  pub fn reset(&mut self) {
    *self = Default::default();
  }

  pub fn has_active(&self) -> bool {
    self.x && self.y && self.z
  }
  pub fn only_x(&self) -> bool {
    self.x && !self.y && !self.z
  }
  pub fn only_y(&self) -> bool {
    !self.x && self.y && !self.z
  }
  pub fn only_z(&self) -> bool {
    !self.x && !self.y && self.z
  }
}

#[derive(Default)]
struct MovingGizmoState {
  active: AxisActiveState,
  last_active_world_position: Vec3<f32>,
}

impl MovingGizmo {
  pub fn new(root: &SceneNode) -> Self {
    fn active(
      active: impl Fn(&mut AxisActiveState),
    ) -> impl Fn(&mut MovingGizmoState, &MouseDown3DEvent) {
      move |state, event| {
        active(&mut state.active);
        state.last_active_world_position = event.world_position;
      }
    };

    let x = build_axis_arrow(root)
      .eventable()
      .on(active(|a| a.x = true));

    let y = build_axis_arrow(root)
      .eventable()
      .on(active(|a| a.y = true));

    let z = build_axis_arrow(root)
      .eventable()
      .on(active(|a| a.z = true));

    let views = vec![x, y, z];

    Self {
      enabled: false,
      states: Default::default(),
      root: root.clone(),
      auto_scale: todo!(),
      view: todo!(),
    }
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    window_state: &WindowState,
    scene: &Scene<WebGPUScene>,
  ) {
    if !self.enabled {
      return;
    }

    let view = self.view.iter_mut().map(|m| m.as_mut());
    map_3d_events(event, view, &mut self.states, info, window_state, scene);
  }
}

impl PassContentWithCamera for &mut MovingGizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if !self.enabled {
      return;
    }

    let dispatcher = &pass.default_dispatcher();

    for c in &self.view {
      c.render(pass, dispatcher, camera)
    }
  }
}

fn interact<'a, T: IntoIterator<Item = &'a mut dyn SceneRenderableShareable>>(
  view: T,
  states: &WindowState,
  info: &CanvasWindowPositionInfo,
  scene: &Scene<WebGPUScene>,
) -> Option<(&'a mut dyn SceneRenderableShareable, MeshBufferHitPoint)> {
  let normalized_position = info.compute_normalized_position_in_canvas_coordinate(states);
  let ray = scene.build_picking_ray_by_view_camera(normalized_position.into());
  interaction_picking_mut(view, ray, &Default::default())
}

pub fn map_3d_events<'a, T: IntoIterator<Item = &'a mut dyn SceneRenderableShareable>>(
  event: &Event<()>,
  view: T,
  user_state: &mut dyn Any,
  info: &CanvasWindowPositionInfo,
  window_state: &WindowState,
  scene: &Scene<WebGPUScene>,
) {
  if let Event::WindowEvent { event, .. } = event {
    match event {
      WindowEvent::KeyboardInput { input, .. } => todo!(),
      WindowEvent::CursorMoved { .. } => {
        if let Some((target, details)) = interact(view, window_state, info, scene) {
          target.event(
            &MouseMove3DEvent {
              world_position: details.hit.position,
            },
            user_state,
          );
        }
      }
      WindowEvent::MouseInput { state, button, .. } => {
        if let Some((target, details)) = interact(view, window_state, info, scene) {
          if *button == MouseButton::Left {
            match state {
              ElementState::Pressed => target.event(
                &MouseDown3DEvent {
                  world_position: details.hit.position,
                },
                user_state,
              ),
              ElementState::Released => todo!(),
            }
          }
        }
      }
      _ => {}
    }
  }
}
