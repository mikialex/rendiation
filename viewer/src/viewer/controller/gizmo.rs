use std::{cell::RefCell, rc::Rc};

use interphaser::{
  winit::event::{ElementState, Event, MouseButton, WindowEvent},
  CanvasWindowPositionInfo, MouseDown, WindowState,
};
use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::tessellation::{CubeMeshParameter, IndexedMeshTessellator};

use crate::*;

pub struct Gizmo {
  scale: AxisScaleGizmo,
  rotation: RotationGizmo,
  translate: MovingGizmo,
}

impl Gizmo {
  pub fn new(root: SceneNode) -> Self {
    let auto_scale = ViewAutoScalable {
      override_position: None,
      independent_scale_factor: 100.,
    };
    let auto_scale = Rc::new(RefCell::new(auto_scale));
    todo!()
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
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  active: AxisActiveState,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
  xy_hint: Box<dyn SceneRenderable>,
  xz_hint: Box<dyn SceneRenderable>,
  zy_hint: Box<dyn SceneRenderable>,
}

fn build_axis_arrow() -> Box<dyn SceneRenderable> {
  todo!();
}

#[derive(Copy, Clone)]
pub struct AxisActiveState {
  x_active: bool,
  y_active: bool,
  z_active: bool,
}

impl AxisActiveState {
  pub fn reset(&mut self) {
    self.x_active = false;
    self.y_active = false;
    self.z_active = false;
  }

  pub fn has_active(&self) -> bool {
    self.x_active && self.y_active && self.z_active
  }
}

impl MovingGizmo {
  fn update_active_state(&mut self, states: &WindowState, info: &CanvasWindowPositionInfo) {
    //
  }
  fn update_target(&mut self, states: &WindowState, info: &CanvasWindowPositionInfo) {
    //
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    states: &WindowState,
  ) {
    if let Event::WindowEvent { event, .. } = event {
      match event {
        WindowEvent::KeyboardInput { input, .. } => todo!(),
        WindowEvent::CursorMoved { .. } => {
          if self.active.has_active() {
            self.update_target(states, info)
          }
        }
        WindowEvent::MouseInput { state, button, .. } => {
          if *button == MouseButton::Left {
            match state {
              ElementState::Pressed => self.update_active_state(states, info),
              ElementState::Released => self.active.reset(),
            }
          }
        }
        _ => {}
      }
    }
  }
}
