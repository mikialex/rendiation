use futures::StreamExt;
use reactive::SignalStreamExt;
use rendiation_geometry::*;
pub use rendiation_texture::Size;

use crate::*;

pub type SceneCamera = SceneItemRef<SceneCameraInner>;

impl SceneCamera {
  pub fn create(projection: CameraProjector, node: SceneNode) -> Self {
    SceneCameraInner {
      bounds: Default::default(),
      projection,
      node,
    }
    .into_ref()
  }

  pub fn create_projection_mat_stream(&self) -> impl Stream<Item = Mat4<f32>> {
    // note, here we have to write like this because we do not have projector change in camera
    // deltas
    self
      .single_listen_by(|view, send| match view {
        MaybeDeltaRef::Delta(delta) => match delta {
          SceneCameraInnerDelta::projection(_) => send(()),
          SceneCameraInnerDelta::node(_) => send(()),
          _ => {}
        },
        MaybeDeltaRef::All(_) => send(()),
      })
      .filter_map_sync(self.defer_weak())
      .map(|camera| camera.read().compute_project_mat())
  }

  pub fn resize(&self, size: (f32, f32)) {
    self.mutate(|mut camera| {
      let resize = CameraProjectorDelta::Resize(size);
      camera.modify(SceneCameraInnerDelta::projection(resize));
    })
  }

  /// normalized_position: -1 to 1
  pub fn cast_world_ray(
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

// /// Manage multi camera view in scene
// pub struct CameraGroup {
//   pub cameras: Vec<SceneCamera>,
//   pub current_rendering_camera: usize,
//   /// if no camera provides, we will use default-camera for handling this case easily.
//   pub default_camera: SceneCamera,
// }

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
pub enum CameraProjector {
  Perspective(PerspectiveProjection<f32>),
  ViewOrthographic(ViewFrustumOrthographicProjection<f32>),
  Orthographic(OrthographicProjection<f32>),
  Foreign(Box<dyn AnyClone + Send + Sync>),
}

pub trait CameraProjection: Sync + Send + DynIncremental {
  /// note, currently we require the implementation use WebGPU NDC
  fn compute_projection_mat(&self) -> Mat4<f32>;
  fn resize(&mut self, size: (f32, f32));
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32>;
}
define_dyn_trait_downcaster_static!(CameraProjection);

impl CameraProjector {
  pub fn compute_projection_mat(&self) -> Option<Mat4<f32>> {
    match self {
      CameraProjector::Perspective(p) => p.compute_projection_mat::<WebGPU>(),
      CameraProjector::ViewOrthographic(p) => p.compute_projection_mat::<WebGPU>(),
      CameraProjector::Orthographic(p) => p.compute_projection_mat::<WebGPU>(),
      CameraProjector::Foreign(p) => get_dyn_trait_downcaster_static!(CameraProjection)
        .downcast_ref(p.as_ref().as_any())?
        .compute_projection_mat(),
    }
    .into()
  }

  pub fn resize(&mut self, size: (f32, f32)) {
    match self {
      CameraProjector::Perspective(p) => p.resize(size),
      CameraProjector::ViewOrthographic(p) => p.resize(size),
      CameraProjector::Orthographic(_) => {}
      CameraProjector::Foreign(_p) => {
        // todo, the arc not support mut
        // if let Some(p) =
        // get_dyn_trait_downcaster_static!(CameraProjection).downcast_mut(p.as_mut()) {
        //   p.resize(size);
        // }
      }
    }
  }

  pub fn pixels_per_unit(&self, distance: f32, view_height: f32) -> Option<f32> {
    match self {
      CameraProjector::Perspective(p) => p.pixels_per_unit(distance, view_height),
      CameraProjector::ViewOrthographic(p) => p.pixels_per_unit(distance, view_height),
      CameraProjector::Orthographic(p) => p.pixels_per_unit(distance, view_height),
      CameraProjector::Foreign(p) => get_dyn_trait_downcaster_static!(CameraProjection)
        .downcast_ref(p.as_ref().as_any())?
        .pixels_per_unit(distance, view_height),
    }
    .into()
  }

  pub fn cast_ray(&self, normalized_position: Vec2<f32>) -> Option<Ray3<f32>> {
    match self {
      CameraProjector::Perspective(p) => p.cast_ray(normalized_position),
      CameraProjector::ViewOrthographic(p) => p.cast_ray(normalized_position),
      CameraProjector::Orthographic(p) => p.cast_ray(normalized_position),
      CameraProjector::Foreign(p) => get_dyn_trait_downcaster_static!(CameraProjection)
        .downcast_ref(p.as_ref().as_any())?
        .cast_ray(normalized_position),
    }
    .into()
  }
}

#[derive(Clone)]
pub enum CameraProjectorDelta {
  Resize((f32, f32)),
  Type(CameraProjector),
}

impl SimpleIncremental for CameraProjector {
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

#[derive(Incremental)]
pub struct SceneCameraInner {
  pub bounds: CameraViewBounds,
  pub projection: CameraProjector,
  pub node: SceneNode,
}

impl AsRef<Self> for SceneCameraInner {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl SceneCameraInner {
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
