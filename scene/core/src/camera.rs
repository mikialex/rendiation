use futures::StreamExt;
use reactive::SignalStreamExt;
use rendiation_geometry::*;
pub use rendiation_texture::Size;

use crate::*;

pub type SceneCamera = IncrementalSignalPtr<SceneCameraImpl>;

#[derive(Incremental)]
pub struct SceneCameraImpl {
  pub bounds: CameraViewBounds,
  pub projection: CameraProjectionEnum,
  pub node: SceneNode,
}

impl SceneCameraImpl {
  pub fn new(projection: CameraProjectionEnum, node: SceneNode) -> Self {
    Self {
      bounds: Default::default(),
      projection,
      node,
    }
  }

  pub fn compute_project_mat(&self) -> Mat4<f32> {
    self
      .projection
      .compute_projection_mat()
      .unwrap_or(Mat4::identity())
  }

  pub fn view_size_in_pixel(&self, frame_size: Size) -> Vec2<f32> {
    let width: usize = frame_size.width.into();
    let width = width as f32 * self.bounds.width;
    let height: usize = frame_size.height.into();
    let height = height as f32 * self.bounds.height;
    (width, height).into()
  }
}

pub trait SceneCameraExt {
  fn create_projection_mat_stream(&self) -> Box<dyn Stream<Item = Mat4<f32>> + Unpin>;

  fn resize(&self, size: (f32, f32));

  /// normalized_position: -1 to 1
  fn cast_world_ray(
    &self,
    normalized_position: Vec2<f32>,
    d_sys: &SceneNodeDeriveSystem,
  ) -> Ray3<f32>;
}

impl SceneCameraExt for SceneCamera {
  // todo remove box
  fn create_projection_mat_stream(&self) -> Box<dyn Stream<Item = Mat4<f32>> + Unpin> {
    // note, here we have to write like this because we do not have projector change in camera
    // deltas
    let s = self
      .single_listen_by(|view, send| match view {
        MaybeDeltaRef::Delta(delta) => match delta {
          SceneCameraImplDelta::projection(_) => send(()),
          SceneCameraImplDelta::node(_) => send(()),
          _ => {}
        },
        MaybeDeltaRef::All(_) => send(()),
      })
      .filter_map_sync(self.defer_weak())
      .map(|camera| camera.read().compute_project_mat());

    Box::new(s)
  }

  fn resize(&self, size: (f32, f32)) {
    self.mutate(|mut camera| {
      let resize = CameraProjectorDelta::Resize(size);
      camera.modify(SceneCameraImplDelta::projection(resize));
    });
  }

  /// normalized_position: -1 to 1
  fn cast_world_ray(
    &self,
    normalized_position: Vec2<f32>,
    d_sys: &SceneNodeDeriveSystem,
  ) -> Ray3<f32> {
    self.visit(|camera| {
      let camera_world_mat = d_sys.get_world_matrix(&camera.node);
      camera
        .projection
        .cast_ray(normalized_position)
        .unwrap_or(Ray3::new(
          Vec3::zero(),
          Vec3::new(1., 0., 0.).into_normalized(),
        ))
        .apply_matrix_into(camera_world_mat)
    })
  }
}

#[derive(Clone, Copy, Incremental)]
pub struct CameraViewBounds {
  pub width: f32,
  pub height: f32,
  pub to_left: f32,
  pub to_top: f32,
}

impl Default for CameraViewBounds {
  fn default() -> Self {
    Self {
      width: 1.,
      height: 1.,
      to_left: 0.,
      to_top: 0.,
    }
  }
}

#[derive(Clone)]
pub enum CameraProjectionEnum {
  Perspective(PerspectiveProjection<f32>),
  ViewOrthographic(ViewFrustumOrthographicProjection<f32>),
  Orthographic(OrthographicProjection<f32>),
  Foreign(ForeignObject),
}

pub trait CameraProjection: Sync + Send + DynIncremental {
  /// note, currently we require the implementation use WebGPU NDC
  fn compute_projection_mat(&self) -> Mat4<f32>;
  fn resize(&mut self, size: (f32, f32));
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32>;
}
define_dyn_trait_downcaster_static!(CameraProjection);

impl CameraProjectionEnum {
  pub fn compute_projection_mat(&self) -> Option<Mat4<f32>> {
    match self {
      CameraProjectionEnum::Perspective(p) => p.compute_projection_mat::<WebGPU>(),
      CameraProjectionEnum::ViewOrthographic(p) => p.compute_projection_mat::<WebGPU>(),
      CameraProjectionEnum::Orthographic(p) => p.compute_projection_mat::<WebGPU>(),
      CameraProjectionEnum::Foreign(p) => get_dyn_trait_downcaster_static!(CameraProjection)
        .downcast_ref(p.as_ref().as_any())?
        .compute_projection_mat(),
    }
    .into()
  }

  pub fn resize(&mut self, size: (f32, f32)) {
    match self {
      CameraProjectionEnum::Perspective(p) => p.resize(size),
      CameraProjectionEnum::ViewOrthographic(p) => p.resize(size),
      CameraProjectionEnum::Orthographic(_) => {}
      CameraProjectionEnum::Foreign(p) => {
        if let Some(p) =
          get_dyn_trait_downcaster_static!(CameraProjection).downcast_mut(p.as_mut().as_any_mut())
        {
          p.resize(size);
        }
      }
    }
  }

  pub fn pixels_per_unit(&self, distance: f32, view_height: f32) -> Option<f32> {
    match self {
      CameraProjectionEnum::Perspective(p) => p.pixels_per_unit(distance, view_height),
      CameraProjectionEnum::ViewOrthographic(p) => p.pixels_per_unit(distance, view_height),
      CameraProjectionEnum::Orthographic(p) => p.pixels_per_unit(distance, view_height),
      CameraProjectionEnum::Foreign(p) => get_dyn_trait_downcaster_static!(CameraProjection)
        .downcast_ref(p.as_ref().as_any())?
        .pixels_per_unit(distance, view_height),
    }
    .into()
  }

  pub fn cast_ray(&self, normalized_position: Vec2<f32>) -> Option<Ray3<f32>> {
    match self {
      CameraProjectionEnum::Perspective(p) => p.cast_ray(normalized_position),
      CameraProjectionEnum::ViewOrthographic(p) => p.cast_ray(normalized_position),
      CameraProjectionEnum::Orthographic(p) => p.cast_ray(normalized_position),
      CameraProjectionEnum::Foreign(p) => get_dyn_trait_downcaster_static!(CameraProjection)
        .downcast_ref(p.as_ref().as_any())?
        .cast_ray(normalized_position),
    }
    .into()
  }
}

#[derive(Clone)]
pub enum CameraProjectorDelta {
  Resize((f32, f32)),
  Type(CameraProjectionEnum),
}

impl SimpleIncremental for CameraProjectionEnum {
  type Delta = CameraProjectorDelta;

  fn s_apply(&mut self, delta: Self::Delta) {
    match delta {
      CameraProjectorDelta::Resize(size) => self.resize(size),
      CameraProjectorDelta::Type(all) => *self = all,
    }
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(CameraProjectorDelta::Type(self.clone()));
  }
}

pub fn camera_projections() -> impl ReactiveCollection<AllocIdx<SceneCameraImpl>, Mat4<f32>> {
  storage_of::<SceneCameraImpl>()
    // extract proj change flag
    .listen_to_reactive_collection(|change| match change {
      MaybeDeltaRef::Delta(delta) => match delta {
        SceneCameraImplDelta::projection(_) => ChangeReaction::Care(Some(AnyChanging)),
        _ => ChangeReaction::NotCare,
      },
      MaybeDeltaRef::All(_) => ChangeReaction::Care(Some(AnyChanging)),
    })
    .collective_execute_simple_map(|camera| camera.compute_project_mat())
}
