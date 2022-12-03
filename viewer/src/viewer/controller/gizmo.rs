use std::{cell::RefCell, marker::PhantomData, rc::Rc, sync::Arc};

use incremental::{DeltaOf, Incremental, SimpleIncremental};
use interphaser::mouse_move;
use rendiation_algebra::*;
use rendiation_geometry::{IntersectAble, OptionalNearest, Plane, Ray3};
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
  deltas: Vec<DeltaOf<GizmoState>>,
  view: Component3DCollection<GizmoState, ()>,
}

impl Gizmo {
  pub fn new(parent: &SceneNode) -> Self {
    let root = &parent.create_child();
    let auto_scale = ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    };
    let auto_scale = &Rc::new(RefCell::new(auto_scale));

    let x_lens = lens_d!(AxisActiveState, x);
    let y_lens = lens_d!(AxisActiveState, y);
    let z_lens = lens_d!(AxisActiveState, z);

    let active_lens = lens_d!(GizmoState, active);
    let translate = lens_d!(GizmoActiveState, translate);

    let translate_x = DeltaChain::new(translate, x_lens);
    let translate_y = DeltaChain::new(translate, y_lens);
    let translate_z = DeltaChain::new(translate, z_lens);

    let active_translate_x = DeltaChain::new(active_lens, translate_x);
    let active_translate_y = DeltaChain::new(active_lens, translate_y);
    let active_translate_z = DeltaChain::new(active_lens, translate_z);

    let x = Arrow::new(root, auto_scale)
      .toward_x()
      .eventable()
      .update(update_arrow(active_translate_x, RED))
      .on(active(active_translate_x));

    let y = Arrow::new(root, auto_scale)
      .toward_y()
      .eventable()
      .update(update_arrow(active_translate_y, BLUE))
      .on(active(active_translate_y));

    let z = Arrow::new(root, auto_scale)
      .toward_z()
      .eventable()
      .update(update_arrow(active_translate_z, GREEN))
      .on(active(active_translate_z));

    macro_rules! duel {
      ($a:tt, $b:tt) => {
        FieldDelta::new::<AxisActiveState, ItemState>(
          |inner_d, cb| {
            // just board cast same change, one changed to all changed.
            cb(DeltaOf::<AxisActiveState>::$a(inner_d.clone()));
            cb(DeltaOf::<AxisActiveState>::$b(inner_d));
          },
          |v, cb| {
            // any source change, compute middle state immediately
            // and expand middle to trigger all change
            if let DeltaOf::<AxisActiveState>::$a(_) | DeltaOf::<AxisActiveState>::$b(_) = v.delta {
              let s = v.data;
              let both = ItemState {
                hovering: s.$a.hovering && s.$b.hovering,
                active: s.$a.active && s.$b.active,
              };

              both.expand(|d| {
                cb(DeltaView {
                  data: &both,
                  delta: &d,
                })
              })
            }
          },
        )
      };
    }

    let xy_lens = duel!(x, y);
    let yz_lens = duel!(y, z);
    let xz_lens = duel!(x, z);

    let xy_lens = DeltaChain::new(translate, xy_lens);
    let yz_lens = DeltaChain::new(translate, yz_lens);
    let xz_lens = DeltaChain::new(translate, xz_lens);

    let xy_lens = DeltaChain::new(active_lens, xy_lens);
    let yz_lens = DeltaChain::new(active_lens, yz_lens);
    let xz_lens = DeltaChain::new(active_lens, xz_lens);

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

    let rotation = lens_d!(GizmoActiveState, rotation);

    let rotation_x = DeltaChain::new(rotation, x_lens);
    let rotation_y = DeltaChain::new(rotation, y_lens);
    let rotation_z = DeltaChain::new(rotation, z_lens);

    let active_rotation_x = DeltaChain::new(active_lens, rotation_x);
    let active_rotation_y = DeltaChain::new(active_lens, rotation_y);
    let active_rotation_z = DeltaChain::new(active_lens, rotation_z);

    let rotator_z = build_rotator(root, auto_scale, Mat4::one())
      .eventable::<GizmoState>()
      .update(update_torus(active_rotation_z, GREEN))
      .on(active(active_rotation_z));
    let rotator_y = build_rotator(root, auto_scale, Mat4::rotate_x(degree_90))
      .eventable::<GizmoState>()
      .update(update_torus(active_rotation_y, BLUE))
      .on(active(active_rotation_y));
    let rotator_x = build_rotator(root, auto_scale, Mat4::rotate_y(degree_90))
      .eventable::<GizmoState>()
      .update(update_torus(active_rotation_x, RED))
      .on(active(active_rotation_x));

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
      deltas: Default::default(),
    }
  }

  pub fn sync_target(&mut self) {
    if let Some(target) = &self.target {
      self.root.set_local_matrix(target.get_world_matrix());
      let sync = GizmoStateDelta::SyncTarget(TargetState {
        target_local_mat: target.get_local_matrix(),
        target_parent_world_mat: target
          .visit_parent(|p| p.world_matrix())
          .unwrap_or_else(Mat4::identity),
        target_world_mat: target.get_world_matrix(),
      });

      self.deltas.push(sync.clone());
      self.states.apply(sync).unwrap();
    }
  }

  pub fn set_target(&mut self, target: Option<SceneNode>) {
    if target.is_none() {
      self.deltas.push(GizmoStateDelta::ReleaseTarget);
      self.states.apply(GizmoStateDelta::ReleaseTarget).unwrap();
    }
    self.target = target;
    self.sync_target();
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
    if self.target.is_some() {
      let mut keep_target = true;

      let mut deltas = Vec::new(); // todo optimize
      self.view.event(&mut self.states, event, &mut |e| {
        if let ViewReaction::StateDelta(d) = e {
          if let GizmoStateDelta::ReleaseTarget = d {
            keep_target = false;
          }
          deltas.push(d);
        }
      });
      deltas
        .iter()
        .for_each(|d| self.states.apply(d.clone()).unwrap());

      self.deltas.extend(deltas);

      #[allow(clippy::collapsible_if)]
      if self.states.start_state.is_some() {
        if mouse_move(event.raw_event).is_some() {
          let screen_position = event
            .info
            .compute_normalized_position_in_canvas_coordinate(event.window_states)
            .into();

          let camera = event.interactive_ctx.camera.read();

          let action = DragTargetAction {
            camera_world: camera.node.get_world_matrix(),
            camera_projection: camera.projection_matrix,
            world_ray: event.interactive_ctx.world_ray,
            screen_position,
          };
          if let Some(target) = &self.states.target_state {
            if let Some(start) = &self.states.start_state {
              if let Some(target_local_new) =
                handle_translating(start, target, &self.states.active.translate, action).or_else(
                  || {
                    handle_rotating(
                      start,
                      target,
                      &mut self.states.rotate_state,
                      &self.states.active.rotation,
                      action,
                    )
                  },
                )
              {
                self
                  .target
                  .as_ref()
                  .unwrap()
                  .set_local_matrix(target_local_new)
              }
            }
          }
        }
      }

      keep_target
    } else {
      false
    }
    .into()
  }

  pub fn update(&mut self) {
    self.sync_target();
    dbg!(self.deltas.len());
    for d in self.deltas.drain(..) {
      self.view.update(&self.states, &d);
    }
  }
}

fn is_3d_hovering() -> impl FnMut(&EventCtx3D) -> Option<bool> {
  let mut is_hovering = false;
  move |event| {
    let mut delta = None; // todo abstraction

    if let Some(event3d) = &event.event_3d {
      if let Event3D::MouseMove { .. } = event3d {
        if !is_hovering {
          delta = Some(true)
        }
        is_hovering = true;
      }
    } else if mouse_move(event.raw_event).is_some() {
      if is_hovering {
        delta = Some(false)
      }
      is_hovering = false;
    }

    delta
  }
}

pub struct DeltaView<'a, T: Incremental> {
  pub data: &'a T,
  pub delta: &'a T::Delta,
}

pub struct DeltaViewMut<'a, T: Incremental> {
  pub data: &'a mut T,
  pub delta: &'a mut T::Delta,
}

pub trait DeltaLens<T: Incremental, U: Incremental> {
  fn map_delta(&self, delta: DeltaOf<U>, cb: &mut dyn FnMut(DeltaOf<T>));
  fn check_delta(&self, delta: DeltaView<T>, cb: &mut dyn FnMut(DeltaView<U>));
}

#[derive(Clone, Copy)]
pub struct FieldDelta<M, C> {
  map_delta: M,
  check_delta: C,
}

#[macro_export]
macro_rules! lens_d {
  ($ty:ty, $field:tt) => {
    $crate::FieldDelta::new::<$ty, _>(
      |inner_d, cb| cb(DeltaOf::<$ty>::$field(inner_d)),
      |v, cb| {
        if let DeltaOf::<$ty>::$field(inner_d) = v.delta {
          cb(DeltaView {
            data: &v.data.$field,
            delta: &inner_d,
          })
        }
      },
    )
  };
}

impl<M, C> FieldDelta<M, C> {
  pub fn new<T, U>(map_delta: M, check_delta: C) -> Self
  where
    T: Incremental,
    U: Incremental,
    M: Fn(DeltaOf<U>, &mut dyn FnMut(DeltaOf<T>)),
    C: Fn(DeltaView<T>, &mut dyn FnMut(DeltaView<U>)),
  {
    Self {
      map_delta,
      check_delta,
    }
  }
}

impl<T, U, M, C> DeltaLens<T, U> for FieldDelta<M, C>
where
  T: Incremental,
  U: Incremental,
  M: Fn(DeltaOf<U>, &mut dyn FnMut(DeltaOf<T>)),
  C: Fn(DeltaView<T>, &mut dyn FnMut(DeltaView<U>)),
{
  fn map_delta(&self, delta: DeltaOf<U>, cb: &mut dyn FnMut(DeltaOf<T>)) {
    (self.map_delta)(delta, cb)
  }

  fn check_delta(&self, delta: DeltaView<T>, cb: &mut dyn FnMut(DeltaView<U>)) {
    (self.check_delta)(delta, cb)
  }
}

pub struct DeltaChain<D1, D2, M> {
  d1: D1,
  middle: PhantomData<M>,
  d2: D2,
}

impl<D1: Clone, D2: Clone, M> Clone for DeltaChain<D1, D2, M> {
  fn clone(&self) -> Self {
    Self {
      d1: self.d1.clone(),
      middle: self.middle,
      d2: self.d2.clone(),
    }
  }
}
impl<D1: Copy, D2: Copy, M> Copy for DeltaChain<D1, D2, M> {}

impl<D1, D2, M> DeltaChain<D1, D2, M> {
  pub fn new(d1: D1, d2: D2) -> Self {
    Self {
      d1,
      middle: PhantomData,
      d2,
    }
  }
}

impl<T, M, U, D1, D2> DeltaLens<T, U> for DeltaChain<D1, D2, M>
where
  T: Incremental,
  M: Incremental,
  U: Incremental,
  D1: DeltaLens<T, M>,
  D2: DeltaLens<M, U>,
{
  fn map_delta(&self, delta: DeltaOf<U>, cb: &mut dyn FnMut(DeltaOf<T>)) {
    self.d2.map_delta(delta, &mut |d| self.d1.map_delta(d, cb))
  }

  fn check_delta(&self, delta: DeltaView<T>, cb: &mut dyn FnMut(DeltaView<U>)) {
    self
      .d1
      .check_delta(delta, &mut |delta| self.d2.check_delta(delta, cb))
  }
}

fn active(
  active: impl DeltaLens<GizmoState, ItemState> + 'static,
) -> impl FnMut(&mut GizmoState, &EventCtx3D, &mut dyn FnMut(DeltaOf<GizmoState>)) + 'static {
  let mut is_hovering = is_3d_hovering();
  move |_, event, cb| {
    if let Some(event3d) = &event.event_3d {
      if let Event3D::MouseDown { world_position } = event3d {
        active.map_delta(DeltaOf::<ItemState>::active(true), cb);
        cb(GizmoStateDelta::StartDrag(*world_position));
      }
    }

    if let Some(hovering) = is_hovering(event) {
      active.map_delta(DeltaOf::<ItemState>::hovering(hovering), cb);
    }
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
  active: impl DeltaLens<GizmoState, ItemState> + 'static,
  color: Vec3<f32>,
) -> impl FnMut(DeltaView<GizmoState>, &mut Arrow) + 'static {
  move |state, arrow| {
    let s = state.data;
    active.check_delta(state, &mut |axis_state| {
      let show = !s.active.translate.has_active() || axis_state.data.active;
      arrow.root.set_visible(show);
      arrow.set_color(map_color(color, *axis_state.data));
    });
  }
}

fn update_plane(
  active: impl DeltaLens<GizmoState, ItemState>,
  color: Vec3<f32>,
) -> impl FnMut(DeltaView<GizmoState>, &mut HelperMesh) {
  move |state, plane| {
    let s = state.data;
    active.check_delta(state, &mut |axis_state| {
      let show = !s.active.translate.has_active() || axis_state.data.active;
      plane.model.node.set_visible(show);
      let color = map_color(color, *axis_state.data);
      plane.material.write().material.color = Vec4::new(color.x, color.y, color.z, 1.);
    });
  }
}

fn update_torus(
  active: impl DeltaLens<GizmoState, ItemState>,
  color: Vec3<f32>,
) -> impl FnMut(DeltaView<GizmoState>, &mut HelperMesh) {
  move |state, torus| {
    let s = state.data;
    active.check_delta(state, &mut |axis_state| {
      let show = !s.active.translate.has_active() || axis_state.data.active;
      torus.model.node.set_visible(show);
      let color = map_color(color, *axis_state.data);
      torus.material.write().material.color = Vec4::new(color.x, color.y, color.z, 1.);
    });
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

fn build_plane(root: &SceneNode, auto_scale: &AutoScale, mat: Mat4<f32>) -> HelperMesh {
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
  let m = material.clone();
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
  let mut model = model.into_matrix_overridable();
  model.push_override(auto_scale.clone());
  HelperMesh { model, material: m }
}

fn build_rotator(root: &SceneNode, auto_scale: &AutoScale, mat: Mat4<f32>) -> HelperMesh {
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
  let m = material.clone();
  let material: Box<dyn WebGPUSceneMaterial> = Box::new(material);
  let material = SceneMaterialType::Foreign(Arc::new(material));

  let torus = root.create_child();

  let model = StandardModel {
    material: material.into(),
    mesh: mesh.into(),
    group: Default::default(),
  };
  let model = SceneModelType::Standard(model.into());
  let model = SceneModelImpl {
    model,
    node: torus.clone(),
  };
  let mut model = model.into_matrix_overridable();

  torus.set_local_matrix(mat);
  model.push_override(auto_scale.clone());
  HelperMesh { model, material: m }
}

fn handle_rotating(
  states: &StartState,
  target: &TargetState,
  rotate_state: &mut Option<RotateState>,
  rotate_view: &AxisActiveState,
  action: DragTargetAction,
) -> Option<Mat4<f32>> {
  // // new_hit_world = M(parent) * M(local_translate) * M(new_local_rotate) * M(local_scale) * start_hit_local_position =>
  // //  M-1(local_translate) * M-1(parent) * new_hit_world =  M(new_local_rotate) * M(local_scale) * start_hit_local_position
  // should we support world space point align like above? but the question is, we have to also modify scale, because
  // it's maybe impossible to rotate one point to the other if your rotation center is origin.

  // here we use simple screen space rotation match local space to see the effects.

  let vp = action.camera_projection * action.camera_world.inverse()?;

  let start_hit_screen_position = (vp * states.start_hit_world_position).xy();
  let pivot_center_screen_position = (vp * target.target_world_mat.position()).xy();

  let origin_dir = start_hit_screen_position - pivot_center_screen_position;
  let origin_dir = origin_dir.normalize();
  let new_dir = action.screen_position - pivot_center_screen_position;
  let new_dir = new_dir.normalize();

  let RotateState {
    current_angle_all,
    last_dir,
  } = rotate_state.get_or_insert_with(|| RotateState {
    current_angle_all: 0.,
    last_dir: origin_dir,
  });

  let rotate_dir = last_dir.cross(new_dir).signum();
  // min one is preventing float precision issue which will cause nan in acos
  let angle_delta = last_dir.dot(new_dir).min(1.).acos() * rotate_dir;

  *current_angle_all += angle_delta;
  *last_dir = new_dir;

  let axis = if rotate_view.only_x_active() {
    Vec3::new(1., 0., 0.)
  } else if rotate_view.only_y_active() {
    Vec3::new(0., 1., 0.)
  } else if rotate_view.only_z_active() {
    Vec3::new(0., 0., 1.)
  } else {
    return None;
  };

  let camera_world_position = action.camera_world.position();

  let view_dir = camera_world_position - target.target_world_mat.position();

  let axis_world = axis.transform_direction(target.target_world_mat);
  let mut angle = *current_angle_all;
  if axis_world.dot(view_dir) < 0. {
    angle = -angle;
  }

  let quat = Quat::rotation(axis, angle);

  let new_local = Mat4::translate(states.start_local_position)
    * Mat4::from(states.start_local_quaternion)
    * Mat4::from(quat)
    * Mat4::scale(states.start_local_scale);

  Some(new_local)
}

fn handle_translating(
  states: &StartState,
  target: &TargetState,
  active: &AxisActiveState,
  action: DragTargetAction,
) -> Option<Mat4<f32>> {
  let camera_world_position = action.camera_world.position();

  let back_to_local = target.target_world_mat.inverse()?;
  let view_dir = camera_world_position - target.target_world_mat.position();
  let view_dir_in_local = view_dir.transform_direction(back_to_local).value;

  let plane_point = states.start_hit_local_position;

  // build world space constraint abstract interactive plane
  let (plane, constraint) = if active.only_x_active() {
    Some((1., 0., 0.).into())
  } else if active.only_y_active() {
    Some((0., 1., 0.).into())
  } else if active.only_z_active() {
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
    if active.only_xy_active() {
      Some((0., 0., 1.).into())
    } else if active.only_yz_active() {
      Some((1., 0., 0.).into())
    } else if active.only_xz_active() {
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
  })?;

  let local_ray = action.world_ray.apply_matrix_into(back_to_local);

  // if we don't get any hit, we skip update.  Keeping last updated result is a reasonable behavior.
  if let OptionalNearest(Some(new_hit)) = local_ray.intersect(&plane, &()) {
    let new_hit = (new_hit.position - plane_point) * constraint + plane_point;
    let new_hit_world = target.target_world_mat * new_hit;

    // new_hit_world = M(parent) * M(new_local_translate) * M(local_rotate) * M(local_scale) * start_hit_local_position =>
    // M-1(parent) * new_hit_world = new_local_translate + M(local_rotate) * M(local_scale) * start_hit_local_position  =>
    // new_local_translate = M-1(parent) * new_hit_world - M(local_rotate) * M(local_scale) * start_hit_local_position

    let new_local_translate = states.start_parent_world_mat.inverse()? * new_hit_world
      - Mat4::from(states.start_local_quaternion)
        * Mat4::scale(states.start_local_scale)
        * states.start_hit_local_position;

    let new_local = Mat4::translate(new_local_translate)
      * Mat4::from(states.start_local_quaternion)
      * Mat4::scale(states.start_local_scale);

    Some(new_local)
  } else {
    None
  }
}

#[derive(Default)]
struct GizmoState {
  active: GizmoActiveState,
  start_state: Option<StartState>,
  rotate_state: Option<RotateState>,
  target_state: Option<TargetState>,
}

#[derive(Default, Incremental)]
struct GizmoActiveState {
  pub translate: AxisActiveState,
  pub rotation: AxisActiveState,
  pub scale: AxisActiveState,
}

#[derive(Copy, Clone)]
struct TargetState {
  target_local_mat: Mat4<f32>,
  target_parent_world_mat: Mat4<f32>,
  target_world_mat: Mat4<f32>,
}

struct RotateState {
  current_angle_all: f32,
  last_dir: Vec2<f32>,
}

struct StartState {
  start_parent_world_mat: Mat4<f32>,
  start_local_position: Vec3<f32>,
  start_local_quaternion: Quat<f32>,
  start_local_scale: Vec3<f32>,
  start_hit_local_position: Vec3<f32>,
  start_hit_world_position: Vec3<f32>,
}

#[derive(Clone, Copy)]
struct DragTargetAction {
  camera_world: Mat4<f32>,
  camera_projection: Mat4<f32>,
  world_ray: Ray3<f32>,
  screen_position: Vec2<f32>,
}

#[derive(Clone)]
#[allow(non_camel_case_types)]
enum GizmoStateDelta {
  active(DeltaOf<GizmoActiveState>),
  StartDrag(Vec3<f32>),
  SyncTarget(TargetState),
  ReleaseTarget,
}

impl SimpleIncremental for GizmoState {
  type Delta = GizmoStateDelta;

  fn s_apply(&mut self, delta: Self::Delta) {
    match delta {
      GizmoStateDelta::StartDrag(start_hit_world_position) => {
        if let Some(target) = self.target_state {
          let (t, r, s) = target.target_local_mat.decompose();
          let start_states = StartState {
            start_parent_world_mat: target.target_parent_world_mat,
            start_local_position: t,
            start_local_quaternion: r,
            start_local_scale: s,
            start_hit_local_position: target.target_world_mat.inverse_or_identity()
              * start_hit_world_position,
            start_hit_world_position,
          };
          self.start_state = Some(start_states);
        }
      }
      GizmoStateDelta::ReleaseTarget => {
        self.start_state = None;
        self.target_state = None;
        self.rotate_state = None;
      }
      GizmoStateDelta::active(delta) => self.active.apply(delta).unwrap(),
      GizmoStateDelta::SyncTarget(s) => self.target_state = Some(s),
    }
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    if let Some(target_state) = &self.target_state {
      cb(GizmoStateDelta::SyncTarget(*target_state));
    }
    self.active.expand(|d| cb(GizmoStateDelta::active(d)));
  }
}

impl GizmoState {
  fn has_any_active(&self) -> bool {
    let state = &self.active;
    state.translate.has_active() || state.rotation.has_active() || state.scale.has_active()
  }
}

#[derive(Copy, Clone, Default, Debug, Incremental)]
pub struct AxisActiveState {
  pub x: ItemState,
  pub y: ItemState,
  pub z: ItemState,
}

#[derive(Copy, Clone, Default, Debug, Incremental)]
pub struct ItemState {
  pub hovering: bool,
  pub active: bool,
}

impl AxisActiveState {
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

struct HelperMesh {
  material: SceneItemRef<StateControl<FlatMaterial>>,
  model: OverridableMeshModelImpl,
}

impl SceneRenderable for HelperMesh {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.model.render(pass, dispatcher, camera)
  }

  fn is_transparent(&self) -> bool {
    self.model.is_transparent()
  }
}

impl SceneRayInteractive for HelperMesh {
  fn ray_pick_nearest(
    &self,
    ctx: &SceneRayInteractiveCtx,
  ) -> OptionalNearest<rendiation_renderable_mesh::MeshBufferHitPoint> {
    self.model.ray_pick_nearest(ctx)
  }
}
