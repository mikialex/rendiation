use std::{cell::RefCell, rc::Rc};

use interphaser::{
  lens, mouse, mouse_move,
  winit::event::{ElementState, MouseButton},
  Component, Lens,
};
use rendiation_algebra::{Mat4, Vec3};
use rendiation_geometry::{OptionalNearest, Ray3};
use rendiation_renderable_mesh::{
  mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig},
  tessellation::{CubeMeshParameter, IndexedMeshTessellator},
};

use crate::{
  helpers::axis::{solid_material, Arrow},
  *,
};

/// Gizmo is a useful widget in 3d design/editor software.
/// User could use this to modify the scene node's transformation.
///
pub struct Gizmo {
  // scale: AxisScaleGizmo,
  // rotation: RotationGizmo,
  translate: TranslateGizmo,
}

impl Gizmo {
  pub fn new(parent: &SceneNode) -> Self {
    let root = parent.create_child();
    let auto_scale = ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    };
    let auto_scale = Rc::new(RefCell::new(auto_scale));
    Self {
      translate: TranslateGizmo::new(&root, &auto_scale),
    }
  }

  pub fn event(&mut self, event: &mut EventCtx3D) {
    self.translate.event(event)
  }

  pub fn update(&mut self) {
    self.translate.update()
  }
}

impl PassContentWithCamera for &mut Gizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    let dispatcher = &pass.default_dispatcher();
    self.translate.render(pass, dispatcher, camera)
  }
}

// pub struct AxisScaleGizmo {
//   pub root: SceneNode,
//   auto_scale: Rc<RefCell<ViewAutoScalable>>,
//   active: AxisActiveState,
//   x: Box<dyn SceneRenderable>,
//   y: Box<dyn SceneRenderable>,
//   z: Box<dyn SceneRenderable>,
// }

// fn build_box() -> Box<dyn SceneRenderable> {
//   let mesh = CubeMeshParameter::default().tessellate();
//   let mesh = MeshCell::new(MeshSource::new(mesh));
//   todo!();
// }

// pub struct RotationGizmo {
//   pub root: SceneNode,
//   auto_scale: Rc<RefCell<ViewAutoScalable>>,
//   active: AxisActiveState,
//   x: Box<dyn SceneRenderable>,
//   y: Box<dyn SceneRenderable>,
//   z: Box<dyn SceneRenderable>,
// }

// fn build_rotation_circle() -> Box<dyn SceneRenderable> {
//   let mut position = Vec::new();
//   let segments = 50;
//   for i in 0..segments {
//     let p = i as f32 / segments as f32;
//     position.push(Vec3::new(p.cos(), p.sin(), 0.))
//   }
//   todo!();
// }

pub struct TranslateGizmo {
  pub enabled: bool,
  states: TranslateGizmoState,
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  view: Component3DCollection<TranslateGizmoState>,
}

fn build_axis_arrow(root: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Arrow {
  let (cylinder, tip) = Arrow::default_shape();
  let (cylinder, tip) = (&cylinder, &tip);
  let material = &solid_material((0.8, 0.1, 0.1));
  Arrow::new_reused(root, auto_scale, material, cylinder, tip)
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
struct TranslateGizmoState {
  active: AxisActiveState,
  last_active_world_position: Vec3<f32>,
}

impl TranslateGizmoState {
  fn show_x(&self) -> bool {
    !self.active.has_active() || self.active.x
  }
  fn show_y(&self) -> bool {
    !self.active.has_active() || self.active.y
  }
  fn show_z(&self) -> bool {
    !self.active.has_active() || self.active.z
  }
}

impl TranslateGizmo {
  pub fn new(root: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Self {
    let x = build_axis_arrow(root, auto_scale)
      .toward_x()
      .eventable::<TranslateGizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.show_x()))
      .on(active(lens!(TranslateGizmoState, active.x)));

    let y = build_axis_arrow(root, auto_scale)
      .toward_y()
      .eventable::<TranslateGizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.show_y()))
      .on(active(lens!(TranslateGizmoState, active.y)));

    let z = build_axis_arrow(root, auto_scale)
      .toward_z()
      .eventable::<TranslateGizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.show_z()))
      .on(active(lens!(TranslateGizmoState, active.z)));

    let view = collection3d().with(x).with(y).with(z);

    Self {
      enabled: false,
      states: Default::default(),
      root: root.clone(),
      auto_scale: auto_scale.clone(),
      view,
    }
  }

  pub fn event(&mut self, event: &mut EventCtx3D) {
    if !self.enabled {
      return;
    }

    // dispatch 3d events into 3d components, handling state active
    self.view.event(&mut self.states, event);

    // after active states get updated, we handling mouse moving in gizmo level
    if mouse_move(event.raw_event).is_some() {
      if !self.states.active.has_active() {
        return;
      }
      // let target_world = self.root.get_world_matrix();

      if self.states.active.only_x() {
        //
      }
      if self.states.active.only_y() {
        //
      }
      if self.states.active.only_z() {
        //
      }
    }
  }
  pub fn update(&mut self) {
    let mut ctx = UpdateCtx3D { placeholder: &() };

    self.view.update(&self.states, &mut ctx);

    self.root.set_local_matrix(Mat4::translate(1., 0., 1.));
  }
}

// this logic mixed with click state handling, try separate it
fn active(
  active: impl Lens<TranslateGizmoState, bool>,
) -> impl FnMut(&mut TranslateGizmoState, &EventCtx3D) {
  let mut is_mouse_down = false;
  move |state, event| {
    if let Some(event3d) = &event.event_3d {
      match event3d {
        Event3D::MouseDown { world_position } => {
          is_mouse_down = true;
          if active.with(state, |active| *active) {
            state.last_active_world_position = *world_position;
          }
        }
        Event3D::MouseUp { .. } => {
          if is_mouse_down {
            active.with_mut(state, |active| {
              *active = true;
            });
          }
        }
        _ => {}
      }
    }

    if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.raw_event) {
      is_mouse_down = false;
    }
  }
}

impl SceneRenderable for TranslateGizmo {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.view.render(pass, dispatcher, camera)
  }
}
