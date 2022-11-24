use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  sync::Arc,
};

use interphaser::{
  lens, mouse, mouse_move,
  winit::event::{ElementState, MouseButton},
  Component, Lens,
};
use rendiation_algebra::*;
use rendiation_geometry::{IntersectAble, OptionalNearest, Plane};
use rendiation_mesh_generator::*;
use rendiation_renderable_mesh::{vertex::Vertex, TriangleList};

use crate::{
  helpers::{
    axis::{solid_material, Arrow},
    WidgetDispatcher,
  },
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

    let x_lens = lens!(GizmoState, translate.x);
    let y_lens = lens!(GizmoState, translate.y);
    let z_lens = lens!(GizmoState, translate.z);

    let x = Arrow::new(root, auto_scale)
      .toward_x()
      .eventable()
      .update(update_arrow(x_lens, RED))
      .on(active(x_lens));

    let y = Arrow::new(root, auto_scale)
      .toward_y()
      .eventable()
      .update(update_arrow(y_lens, BLUE))
      .on(active(y_lens));

    let z = Arrow::new(root, auto_scale)
      .toward_z()
      .eventable()
      .update(update_arrow(z_lens, GREEN))
      .on(active(z_lens));

    macro_rules! duel {
      ($a:tt, $b:tt) => {
        interphaser::Map::new(
          |s: &GizmoState| ItemState {
            hovering: s.translate.$a.hovering && s.translate.$b.hovering,
            active: s.translate.$a.active && s.translate.$b.active,
          },
          |s, v| {
            let both = ItemState {
              hovering: !(s.translate.$a.hovering ^ s.translate.$b.hovering),
              active: !(s.translate.$a.active ^ s.translate.$b.active),
            };
            if both.hovering {
              s.translate.$a.hovering = v.hovering;
              s.translate.$b.hovering = v.hovering;
            }
            if both.active {
              s.translate.$a.active = v.active;
              s.translate.$b.active = v.active;
            }
          },
        )
      };
    }

    let xy_lens = duel!(x, y);
    let yz_lens = duel!(y, z);
    let xz_lens = duel!(x, z);

    let plane_scale = Mat4::scale(Vec3::splat(0.4));
    let plane_move = Vec3::splat(1.3);
    let degree_90 = f32::PI() / 2.;

    let xy_t = Vec3::new(1., 1., 0.);
    let xy_t = Mat4::translate(xy_t * plane_move) * plane_scale;
    let xy = build_plane(root, auto_scale, xy_t)
      .eventable::<GizmoState>()
      .update(update_plane(xy_lens, GREEN))
      .on(active(xy_lens));

    let yz_t = Vec3::new(0., 1., 1.);
    let yz_t = Mat4::translate(yz_t * plane_move) * Mat4::rotate_y(degree_90) * plane_scale;
    let yz = build_plane(root, auto_scale, yz_t)
      .eventable::<GizmoState>()
      .update(update_plane(yz_lens, RED))
      .on(active(yz_lens));

    let xz_t = Vec3::new(1., 0., 1.);
    let xz_t = Mat4::translate(xz_t * plane_move) * Mat4::rotate_x(-degree_90) * plane_scale;
    let xz = build_plane(root, auto_scale, xz_t)
      .eventable::<GizmoState>()
      .update(update_plane(xz_lens, BLUE))
      .on(active(xz_lens));

    let x_lens = lens!(GizmoState, rotation.x);
    let y_lens = lens!(GizmoState, rotation.y);
    let z_lens = lens!(GizmoState, rotation.z);

    let rotator_z = build_rotator(root, auto_scale, Mat4::one())
      .eventable::<GizmoState>()
      .update(update_torus(z_lens, GREEN))
      .on(active(z_lens));
    let rotator_y = build_rotator(root, auto_scale, Mat4::rotate_x(degree_90))
      .eventable::<GizmoState>()
      .update(update_torus(y_lens, BLUE))
      .on(active(y_lens));
    let rotator_x = build_rotator(root, auto_scale, Mat4::rotate_y(degree_90))
      .eventable::<GizmoState>()
      .update(update_torus(x_lens, RED))
      .on(active(x_lens));

    #[rustfmt::skip]
    let view = collection3d()
      .with(x).with(y).with(z)
      .with(xy).with(yz).with(xz)
      .with(rotator_x)
      .with(rotator_y)
      .with(rotator_z);

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
    self.states.has_any_active()
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
        .visit_parent(|p| p.world_matrix())
        .unwrap_or_else(Mat4::identity);

      if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.raw_event) {
        self.states.test_has_any_widget_mouse_down = false;
      }

      self.view.event(&mut self.states, event);

      if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.raw_event) {
        if !self.states.test_has_any_widget_mouse_down {
          keep_target = false;
          self.states.reset();
        }
      }

      if !self.states.has_any_active() {
        return keep_target.into();
      }

      // after active states get updated, we handling mouse moving in gizmo level
      if mouse_move(event.raw_event).is_some() {
        self.handle_translating(event, target)?;
        self.handle_rotating(event, target)?;
      }

      if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.raw_event) {
        self.states.reset();
      }

      keep_target
    } else {
      false
    }
    .into()
  }

  fn handle_rotating(&self, event: &mut EventCtx3D, target: &SceneNode) -> Option<()> {
    // // new_hit_world = M(parent) * M(local_translate) * M(new_local_rotate) * M(local_scale) * start_hit_local_position =>
    // //  M-1(local_translate) * M-1(parent) * new_hit_world =  M(new_local_rotate) * M(local_scale) * start_hit_local_position
    // should we support world space point align like above? but the question is, we have to also modify scale, because
    // it's maybe impossible to rotate one point to the other if your rotation center is origin.

    // here we use simple screen space rotation match local space to see the effects.

    let camera = event.interactive_ctx.camera.read();
    let vp = camera.projection_matrix * camera.node.get_world_matrix().inverse()?;

    let start_hit_screen_position = (vp * self.states.start_hit_world_position).xy();
    let current_hit_screen_position: Vec2<f32> = event
      .info
      .compute_normalized_position_in_canvas_coordinate(event.window_states)
      .into();
    let pivot_center_screen_position = (vp * self.states.target_world_mat.position()).xy();

    let origin_dir = start_hit_screen_position - pivot_center_screen_position;
    let origin_dir = origin_dir.normalize();
    let new_dir = current_hit_screen_position - pivot_center_screen_position;
    let new_dir = new_dir.normalize();

    let current_angle_all = self.states.current_angle_all.get().unwrap_or(0.);
    let last_dir = self.states.last_dir.get().unwrap_or(origin_dir);

    let rotate_dir = last_dir.cross(new_dir).signum();
    // min one is preventing float precision issue which will cause nan in acos
    let angle_delta = last_dir.dot(new_dir).min(1.).acos() * rotate_dir;
    let mut angle = current_angle_all + angle_delta;

    self.states.current_angle_all.set(Some(angle));
    self.states.last_dir.set(Some(new_dir));

    let axis = if self.states.rotation.only_x_active() {
      Vec3::new(1., 0., 0.)
    } else if self.states.rotation.only_y_active() {
      Vec3::new(0., 1., 0.)
    } else if self.states.rotation.only_z_active() {
      Vec3::new(0., 0., 1.)
    } else {
      return Some(());
    };

    let camera_world_position = event
      .interactive_ctx
      .camera
      .read()
      .node
      .get_world_matrix()
      .position();

    let view_dir = camera_world_position - self.states.target_world_mat.position();

    let axis_world = axis.transform_direction(self.states.target_world_mat);
    if axis_world.dot(view_dir) < 0. {
      angle = -angle;
    }

    let quat = Quat::rotation(axis, angle);

    let new_local = Mat4::translate(self.states.start_local_position)
      * Mat4::from(self.states.start_local_quaternion)
      * Mat4::from(quat)
      * Mat4::scale(self.states.start_local_scale);

    target.set_local_matrix(new_local);
    self.root.set_local_matrix(new_local);

    Some(())
  }

  fn handle_translating(&self, event: &mut EventCtx3D, target: &SceneNode) -> Option<()> {
    let camera_world_position = event
      .interactive_ctx
      .camera
      .read()
      .node
      .get_world_matrix()
      .position();

    let back_to_local = self.states.target_world_mat.inverse()?;
    let view_dir = camera_world_position - self.states.target_world_mat.position();
    let view_dir_in_local = view_dir.transform_direction(back_to_local).value;

    let plane_point = self.states.start_hit_local_position;

    // build world space constraint abstract interactive plane
    let (plane, constraint) = if self.states.translate.only_x_active() {
      Some((1., 0., 0.).into())
    } else if self.states.translate.only_y_active() {
      Some((0., 1., 0.).into())
    } else if self.states.translate.only_z_active() {
      Some((0., 0., 1.).into())
    } else {
      None
    }
    .map(|axis: Vec3<f32>| {
      let helper_dir = axis.cross(view_dir_in_local);
      let normal = helper_dir.cross(axis);
      (
        Plane::from_normal_and_plane_point(normal, plane_point),
        axis,
      )
    })
    .or_else(|| {
      if self.states.translate.only_xy_active() {
        Some((0., 0., 1.).into())
      } else if self.states.translate.only_yz_active() {
        Some((1., 0., 0.).into())
      } else if self.states.translate.only_xz_active() {
        Some((0., 1., 0.).into())
      } else {
        None
      }
      .map(|normal: Vec3<f32>| {
        (
          Plane::from_normal_and_plane_point(normal, plane_point),
          Vec3::one() - normal,
        )
      })
    })
    .unwrap_or((
      // should be unreachable
      Plane::from_normal_and_origin_point((0., 1., 0.).into()),
      (0., 1., 0.).into(),
    ));

    let local_ray = event
      .interactive_ctx
      .world_ray
      .apply_matrix_into(back_to_local);

    // if we don't get any hit, we skip update.  Keeping last updated result is a reasonable behavior.
    if let OptionalNearest(Some(new_hit)) = local_ray.intersect(&plane, &()) {
      let new_hit = (new_hit.position - plane_point) * constraint + plane_point;
      let new_hit_world = self.states.target_world_mat * new_hit;

      // new_hit_world = M(parent) * M(new_local_translate) * M(local_rotate) * M(local_scale) * start_hit_local_position =>
      // M-1(parent) * new_hit_world = new_local_translate + M(local_rotate) * M(local_scale) * start_hit_local_position  =>
      // new_local_translate = M-1(parent) * new_hit_world - M(local_rotate) * M(local_scale) * start_hit_local_position

      let new_local_translate = self.states.start_parent_world_mat.inverse()? * new_hit_world
        - Mat4::from(self.states.start_local_quaternion)
          * Mat4::scale(self.states.start_local_scale)
          * self.states.start_hit_local_position;

      let new_local = Mat4::translate(new_local_translate)
        * Mat4::from(self.states.start_local_quaternion)
        * Mat4::scale(self.states.start_local_scale);

      target.set_local_matrix(new_local);
      self.root.set_local_matrix(new_local);
    }

    Some(())
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

fn active(active: impl Lens<GizmoState, ItemState>) -> impl FnMut(&mut GizmoState, &EventCtx3D) {
  let mut is_hovering = is_3d_hovering();
  move |state, event| {
    if let Some(event3d) = &event.event_3d {
      if let Event3D::MouseDown { world_position } = event3d {
        active.with_mut(state, |s| s.active = true);
        state.test_has_any_widget_mouse_down = true;
        state.record_start(*world_position)
      }
    }

    active.with_mut(state, |s| s.hovering = is_hovering(event));
  }
}

fn map_color(color: Vec3<f32>, state: ItemState) -> Vec3<f32> {
  if state.hovering && !state.active {
    color + Vec3::splat(0.1)
  } else if state.active {
    color - Vec3::splat(0.1)
  } else {
    color
  }
}

fn update_arrow(
  active: impl Lens<GizmoState, ItemState>,
  color: Vec3<f32>,
) -> impl FnMut(&GizmoState, &mut Arrow) {
  move |state, arrow| {
    let axis_state = active.with(state, |&s| s);
    let show = !state.translate.has_active() || axis_state.active;
    arrow.root.set_visible(show);
    arrow.set_color(map_color(color, axis_state));
  }
}

fn update_plane(
  active: impl Lens<GizmoState, ItemState>,
  color: Vec3<f32>,
) -> impl FnMut(&GizmoState, &mut PlaneModel) {
  move |state, plane| {
    let axis_state = active.with(state, |&s| s);
    let color = map_color(color, axis_state);
    // plane.material.write().material.color = Vec4::new(color.x, color.y, color.z, 1.);
    let show = !state.translate.has_active() || axis_state.active;
    plane.node.set_visible(show);
  }
}

fn update_torus(
  active: impl Lens<GizmoState, ItemState>,
  color: Vec3<f32>,
) -> impl FnMut(&GizmoState, &mut RotatorModel) {
  move |state, torus| {
    let axis_state = active.with(state, |&s| s);
    let color = map_color(color, axis_state);

    // if let  SceneModelType::Foreign(model) = torus {

    // }

    // torus.material.write().material.color = Vec4::new(color.x, color.y, color.z, 1.);
    // let show = !state.translate.has_active() || axis_state.active;
    // torus.node.set_visible(show);
  }
}

impl PassContentWithCamera for &mut Gizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if self.target.is_none() {
      return;
    }

    let dispatcher = &WidgetDispatcher::new(pass.default_dispatcher());
    self.view.render(pass, dispatcher, camera)
  }
}

type AutoScale = Rc<RefCell<ViewAutoScalable>>;

type FlatUtilMaterial = StateControl<FlatMaterial>;
type PlaneModel = OverridableMeshModelImpl;
fn build_plane(root: &SceneNode, auto_scale: &AutoScale, mat: Mat4<f32>) -> PlaneModel {
  let mesh = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
    .triangulate_parametric(
      &ParametricPlane.transform_by(Mat4::translate((-0.5, -0.5, 0.))),
      TessellationConfig { u: 1, v: 1 },
      true,
    )
    .build_mesh_into();

  let mesh = MeshSource::new(mesh);
  let mesh = SceneItemRef::new(mesh);
  let mesh: Box<dyn WebGPUSceneMesh> = Box::new(mesh);
  let mesh = SceneMeshType::Foreign(Arc::new(mesh));

  let material = solid_material(RED);
  let material = SceneItemRef::new(material);
  let material: Box<dyn WebGPUSceneMaterial> = Box::new(material);
  let material = SceneMaterialType::Foreign(Arc::new(material));

  let plane = root.create_child();

  plane.set_local_matrix(mat);

  let model = StandardModel {
    material: material.into(),
    mesh: mesh.into(),
    group: Default::default(),
  };
  let model = SceneModelType::Standard(model.into());
  let model = SceneModelImpl { model, node: plane };
  let model = model.into_matrix_overridable();
  model.push_override(auto_scale.clone());
  model
}

type RotatorModel = OverridableMeshModelImpl;
fn build_rotator(root: &SceneNode, auto_scale: &AutoScale, mat: Mat4<f32>) -> RotatorModel {
  let mesh = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
    .triangulate_parametric(
      &TorusMeshParameter {
        radius: 1.5,
        tube_radius: 0.03,
      }
      .make_surface(),
      TessellationConfig { u: 36, v: 4 },
      true,
    )
    .build_mesh_into();

  let mesh = MeshSource::new(mesh);
  let mesh = SceneItemRef::new(mesh);
  let mesh: Box<dyn WebGPUSceneMesh> = Box::new(mesh);
  let mesh = SceneMeshType::Foreign(Arc::new(mesh));

  let material = solid_material(RED);
  let material = SceneItemRef::new(material);
  let material: Box<dyn WebGPUSceneMaterial> = Box::new(material);
  let material = SceneMaterialType::Foreign(Arc::new(material));

  let torus = root.create_child();

  let model = StandardModel {
    material: material.into(),
    mesh: mesh.into(),
    group: Default::default(),
  };
  let model = SceneModelType::Standard(model.into());
  let model = SceneModelImpl { model, node: torus };
  let model = model.into_matrix_overridable();

  // (SceneModelImpl{ model: todo!(), node: todo!() }::new(material, mesh).into_matrix_overridable())

  torus.set_local_matrix(mat);
  model.push_override(auto_scale.clone());
  model
}

// fn build_box() -> Box<dyn SceneRenderable> {
//   let mesh = CubeMeshParameter::default().tessellate();
//   let mesh = MeshCell::new(MeshSource::new(mesh));
//   todo!();
// }

#[derive(Default)]
struct GizmoState {
  translate: AxisActiveState,
  rotation: AxisActiveState,
  scale: AxisActiveState,
  start_parent_world_mat: Mat4<f32>,
  start_local_position: Vec3<f32>,
  start_local_quaternion: Quat<f32>,
  start_local_scale: Vec3<f32>,
  start_local_mat: Mat4<f32>,
  start_hit_local_position: Vec3<f32>,
  start_hit_world_position: Vec3<f32>,

  current_angle_all: Cell<Option<f32>>,
  last_dir: Cell<Option<Vec2<f32>>>,

  target_local_mat: Mat4<f32>,
  target_parent_world_mat: Mat4<f32>,
  target_world_mat: Mat4<f32>,
  test_has_any_widget_mouse_down: bool,
}

impl GizmoState {
  fn has_any_active(&self) -> bool {
    self.translate.has_active() || self.rotation.has_active() || self.scale.has_active()
  }
  fn reset(&mut self) {
    self.translate.reset_active();
    self.rotation.reset_active();
    self.scale.reset_active();

    self.last_dir.set(None);
    self.current_angle_all.set(None);
  }
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
  x: ItemState,
  y: ItemState,
  z: ItemState,
}

#[derive(Copy, Clone, Default, Debug)]
struct ItemState {
  hovering: bool,
  active: bool,
}

impl AxisActiveState {
  pub fn reset_active(&mut self) {
    self.x.active = false;
    self.y.active = false;
    self.z.active = false;
  }

  pub fn has_active(&self) -> bool {
    self.x.active || self.y.active || self.z.active
  }
  pub fn only_x_active(&self) -> bool {
    self.x.active && !self.y.active && !self.z.active
  }
  pub fn only_y_active(&self) -> bool {
    !self.x.active && self.y.active && !self.z.active
  }
  pub fn only_z_active(&self) -> bool {
    !self.x.active && !self.y.active && self.z.active
  }
  pub fn only_xy_active(&self) -> bool {
    self.x.active && self.y.active && !self.z.active
  }
  pub fn only_yz_active(&self) -> bool {
    !self.x.active && self.y.active && self.z.active
  }
  pub fn only_xz_active(&self) -> bool {
    self.x.active && !self.y.active && self.z.active
  }
}
