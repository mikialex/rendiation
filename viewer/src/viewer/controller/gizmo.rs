use std::{cell::RefCell, rc::Rc};

use interphaser::{
  lens, mouse, mouse_move,
  winit::event::{ElementState, MouseButton},
  Component, Lens,
};
use rendiation_algebra::*;
use rendiation_geometry::{IntersectAble, OptionalNearest, Plane};
// use rendiation_geometry::{OptionalNearest, Ray3};
// use rendiation_renderable_mesh::{
//   mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig},
//   tessellation::{CubeMeshParameter, IndexedMeshTessellator},
// };

use crate::{
  helpers::axis::{solid_material, Arrow},
  *,
};

/// Gizmo is a useful widget in 3d design/editor software.
/// User could use this to modify the scene node's transformation.
///
pub struct Gizmo {
  states: GizmoState,
  root: SceneNode,
  target: Option<SceneNode>,
  view: Component3DCollection<GizmoState>,
}

impl Gizmo {
  pub fn new(parent: &SceneNode) -> Self {
    let root = &parent.create_child();
    let auto_scale = ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    };
    let auto_scale = &Rc::new(RefCell::new(auto_scale));
    let x = build_axis_arrow(root, auto_scale)
      .toward_x()
      .eventable::<GizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.show_x()))
      .on(active(lens!(GizmoState, active.x)));

    let y = build_axis_arrow(root, auto_scale)
      .toward_y()
      .eventable::<GizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.show_y()))
      .on(active(lens!(GizmoState, active.y)));

    let z = build_axis_arrow(root, auto_scale)
      .toward_z()
      .eventable::<GizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.show_z()))
      .on(active(lens!(GizmoState, active.z)));

    let view = collection3d().with(x).with(y).with(z);

    Self {
      states: Default::default(),
      root: root.clone(),
      view,
      target: None,
    }
  }

  pub fn set_target(&mut self, target: Option<SceneNode>) {
    self.target = target;
  }

  pub fn has_target(&self) -> bool {
    self.target.is_some()
  }
  pub fn has_active(&self) -> bool {
    self.states.active.has_active()
  }

  // return if should keep target.
  pub fn event(&mut self, event: &mut EventCtx3D) -> bool {
    // we don't want handle degenerate case by just using identity fallback but do early return
    self.event_impl(event).unwrap_or_else(|| {
      log::error!("failed to apply gizmo control maybe because of degenerate transform");
      false
    })
  }
  // return if should keep target.
  pub fn event_impl(&mut self, event: &mut EventCtx3D) -> Option<bool> {
    if let Some(target) = &self.target {
      let mut keep_target = true;

      // dispatch 3d events into 3d components, handling state active
      self.states.target_world_mat = self.root.get_world_matrix();
      self.states.target_local_mat = target.get_local_matrix();
      self.states.target_parent_world_mat = target
        .visit_parent(|p| p.world_matrix)
        .unwrap_or_else(Mat4::identity);

      if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.raw_event) {
        self.states.test_has_any_widget_mouse_down = false;
      }

      self.view.event(&mut self.states, event);

      if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.raw_event) {
        if !self.states.test_has_any_widget_mouse_down {
          keep_target = false;
          self.states.active.reset();
        }
      }

      if !self.states.active.has_active() {
        return keep_target.into();
      }

      // after active states get updated, we handling mouse moving in gizmo level
      if mouse_move(event.raw_event).is_some() {
        let camera_world_position = event
          .interactive_ctx
          .camera
          .node
          .get_world_matrix()
          .position();

        let view = camera_world_position - self.states.target_world_mat.position();

        let (plane, constraint) = if self.states.active.only_x() {
          let x = Vec3::new(1., 0., 0.);
          let helper_dir = x.cross(view);
          let normal = helper_dir.cross(x);
          let plane = Plane::from_normal_and_origin_point(normal);
          (plane, x)
        } else if self.states.active.only_y() {
          let y = Vec3::new(0., 1., 0.);
          let helper_dir = y.cross(view);
          let normal = helper_dir.cross(y);
          let plane = Plane::from_normal_and_origin_point(normal);
          (plane, y)
        } else if self.states.active.only_z() {
          let z = Vec3::new(0., 0., 1.);
          let helper_dir = z.cross(view);
          let normal = helper_dir.cross(z);
          let plane = Plane::from_normal_and_origin_point(normal);
          (plane, z)
        } else {
          let y = Vec3::new(0., 1., 0.);
          let plane = Plane::from_normal_and_origin_point(y);
          (plane, y)
        };

        // if we don't get any hit, we skip update.  Keeping last updated result is a reasonable behavior.
        if let OptionalNearest(Some(new_hit)) =
          event.interactive_ctx.world_ray.intersect(&plane, &())
        {
          let new_hit = new_hit.position * constraint;

          let new_local_translate = Mat4::from(self.states.start_local_quaternion).inverse()?
            * Mat4::scale(self.states.start_local_scale).inverse()?
            * self.states.start_parent_world_mat.inverse()?
            * new_hit
            - self.states.start_hit_local_position
            - self.states.start_local_position;

          target.set_local_matrix(Mat4::translate(new_local_translate));

          self
            .root
            .set_local_matrix(Mat4::translate(new_local_translate));
        }
      }

      if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.raw_event) {
        self.states.active.reset();
      }

      keep_target
    } else {
      false
    }
    .into()
  }

  pub fn update(&mut self) {
    if self.target.is_some() {
      let mut ctx = UpdateCtx3D { placeholder: &() };

      self.view.update(&self.states, &mut ctx);
    }
  }
}

fn active(active: impl Lens<GizmoState, bool>) -> impl FnMut(&mut GizmoState, &EventCtx3D) {
  move |state, event| {
    if let Some(event3d) = &event.event_3d {
      if let Event3D::MouseDown { world_position } = event3d {
        active.with_mut(state, |active| *active = true);
        state.test_has_any_widget_mouse_down = true;
        state.record_start(*world_position)
      }
    }
  }
}

impl PassContentWithCamera for &mut Gizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if self.target.is_none() {
      return;
    }

    let dispatcher = &pass.default_dispatcher();
    self.view.render(pass, dispatcher, camera)
  }
}

// fn build_box() -> Box<dyn SceneRenderable> {
//   let mesh = CubeMeshParameter::default().tessellate();
//   let mesh = MeshCell::new(MeshSource::new(mesh));
//   todo!();
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

fn build_axis_arrow(root: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Arrow {
  let (cylinder, tip) = Arrow::default_shape();
  let (cylinder, tip) = (&cylinder, &tip);
  let material = &solid_material((0.8, 0.1, 0.1));
  Arrow::new_reused(root, auto_scale, material, cylinder, tip)
}

#[derive(Default)]
struct GizmoState {
  active: AxisActiveState,

  start_parent_world_mat: Mat4<f32>,
  start_local_position: Vec3<f32>,
  start_local_quaternion: Quat<f32>,
  start_local_scale: Vec3<f32>,
  start_local_mat: Mat4<f32>,
  start_hit_local_position: Vec3<f32>,
  start_hit_world_position: Vec3<f32>,

  target_local_mat: Mat4<f32>,
  target_parent_world_mat: Mat4<f32>,
  target_world_mat: Mat4<f32>,
  test_has_any_widget_mouse_down: bool,
}

impl GizmoState {
  fn record_start(&mut self, start_hit_world_position: Vec3<f32>) {
    self.start_local_mat = self.target_local_mat;
    self.start_parent_world_mat = self.target_parent_world_mat;

    let (t, r, s) = self.start_local_mat.decompose();
    self.start_local_position = t;
    self.start_local_quaternion = r;
    self.start_local_scale = s;

    self.start_hit_world_position = start_hit_world_position;
    self.start_hit_local_position =
      self.start_local_mat.inverse_or_identity() * self.start_hit_world_position;
  }

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

#[derive(Copy, Clone, Default, Debug)]
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
    self.x || self.y || self.z
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
