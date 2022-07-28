use std::{cell::RefCell, rc::Rc};

use interphaser::{
  lens, mouse, mouse_move,
  winit::event::{ElementState, MouseButton},
  Component, Lens,
};
use rendiation_algebra::*;
use rendiation_geometry::{IntersectAble, OptionalNearest, Plane};
use rendiation_renderable_mesh::tessellation::{IndexedMeshTessellator, PlaneMeshParameter};

use crate::{
  helpers::axis::{solid_material, Arrow},
  *,
};

const RED: Vec3<f32> = Vec3::new(0.8, 0.1, 0.1);
const GREEN: Vec3<f32> = Vec3::new(0.1, 0.8, 0.1);
const BLUE: Vec3<f32> = Vec3::new(0.1, 0.1, 0.8);

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

    let x_lens = lens!(GizmoState, active.x);
    let y_lens = lens!(GizmoState, active.y);
    let z_lens = lens!(GizmoState, active.z);

    let x = Arrow::new(root, auto_scale)
      .toward_x()
      .eventable()
      .update(update(x_lens, RED))
      .on(active(x_lens));

    let y = Arrow::new(root, auto_scale)
      .toward_y()
      .eventable()
      .update(update(y_lens, BLUE))
      .on(active(y_lens));

    let z = Arrow::new(root, auto_scale)
      .toward_z()
      .eventable()
      .update(update(z_lens, GREEN))
      .on(active(z_lens));

    let xy = build_plane(root, auto_scale, Mat4::translate((1., 1., 0.))).eventable::<GizmoState>();
    let yz = build_plane(
      root,
      auto_scale,
      Mat4::translate((0., 1., 1.)) * Mat4::rotate_y(f32::PI() / 2.),
    )
    .eventable::<GizmoState>();
    let xz = build_plane(
      root,
      auto_scale,
      Mat4::translate((1., 0., 1.)) * Mat4::rotate_x(f32::PI() / 2.),
    )
    .eventable::<GizmoState>();

    #[rustfmt::skip]
    let view = collection3d()
      .with(x).with(y).with(z)
      .with(xy).with(yz).with(xz);

    Self {
      states: Default::default(),
      root: root.clone(),
      view,
      target: None,
    }
  }

  pub fn set_target(&mut self, target: Option<SceneNode>) {
    if let Some(target) = &target {
      self.root.set_local_matrix(target.get_world_matrix())
    }
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
          .read()
          .node
          .get_world_matrix()
          .position();

        let target_world_position = self.states.target_world_mat.position();
        let view = camera_world_position - target_world_position;

        let plane_point = self.states.start_hit_world_position;

        let axis: Vec3<_> = if self.states.active.only_x() {
          (1., 0., 0.)
        } else if self.states.active.only_y() {
          (0., 1., 0.)
        } else if self.states.active.only_z() {
          (0., 0., 1.)
        } else {
          (0., 1., 0.)
        }
        .into();

        // build world space constraint abstract interactive plane
        let helper_dir = axis.cross(view);
        let normal = helper_dir.cross(axis);
        let plane = Plane::from_normal_and_plane_point(normal, plane_point);

        // if we don't get any hit, we skip update.  Keeping last updated result is a reasonable behavior.
        if let OptionalNearest(Some(new_hit)) =
          event.interactive_ctx.world_ray.intersect(&plane, &())
        {
          let new_hit = (new_hit.position - plane_point) * axis + plane_point;

          // new_hit_world = M(parent) * M(new_local_translate) * M(local_rotate) * M(local_scale) * start_hit_local_position =>
          // M-1(parent) * new_hit_world = new_local_translate + M(local_rotate) * M(local_scale) * start_hit_local_position  =>
          // new_local_translate = M-1(parent) * new_hit_world - M(local_rotate) * M(local_scale) * start_hit_local_position
          let new_local_translate = self.states.start_parent_world_mat.inverse()? * new_hit
            - Mat4::from(self.states.start_local_quaternion)
              * Mat4::scale(self.states.start_local_scale)
              * self.states.start_hit_local_position;

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

fn is_3d_hovering() -> impl FnMut(&EventCtx3D) -> bool {
  let mut is_hovering = false;
  move |event| {
    if let Some(event3d) = &event.event_3d {
      if let Event3D::MouseMove { .. } = event3d {
        is_hovering = true;
      }
    } else if mouse_move(event.raw_event).is_some() {
      is_hovering = false;
    }

    is_hovering
  }
}

fn active(active: impl Lens<GizmoState, bool>) -> impl FnMut(&mut GizmoState, &EventCtx3D) {
  let mut is_hovering = is_3d_hovering();
  move |state, event| {
    if let Some(event3d) = &event.event_3d {
      if let Event3D::MouseDown { world_position } = event3d {
        active.with_mut(state, |active| *active = true);
        state.test_has_any_widget_mouse_down = true;
        state.record_start(*world_position)
      }
    }

    if is_hovering(event) {
      println!("hovering")
    }
  }
}

fn update(
  active: impl Lens<GizmoState, bool>,
  color: Vec3<f32>,
) -> impl FnMut(&GizmoState, &mut Arrow) {
  move |state, arrow| {
    let axis_active = active.with(state, |&s| s);
    let show = !state.active.has_active() || axis_active;
    arrow.set_color(color);
    arrow.root.set_visible(show);
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

type PlaneMaterial = StateControl<FlatMaterial>;
type PlaneMesh = impl WebGPUMesh;
fn build_plane(
  root: &SceneNode,
  auto_scale: &Rc<RefCell<ViewAutoScalable>>,
  mat: Mat4<f32>,
) -> OverridableMeshModelImpl<PlaneMesh, PlaneMaterial> {
  let mesh = PlaneMeshParameter::default().tessellate();
  let mesh = MeshSource::new(mesh);

  let material = solid_material(RED);

  let plane = root.create_child();
  plane.set_local_matrix(mat);
  let mut plane = MeshModelImpl::new(material, mesh, plane).into_matrix_overridable();

  plane.push_override(auto_scale.clone());
  plane
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
      self.target_world_mat.inverse_or_identity() * self.start_hit_world_position;
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
