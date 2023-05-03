use rendiation_geometry::*;
use rendiation_texture::Size;

use crate::*;

pub type SceneCamera = SceneItemRef<SceneCameraInner>;

impl SceneCamera {
  pub fn create_camera(
    p: impl ResizableProjection<f32> + RayCaster3<f32> + DynIncremental + Clone + 'static,
    node: SceneNode,
  ) -> Self {
    let mut inner = SceneCameraInner {
      bounds: Default::default(),
      projection: Box::new(p),
      projection_matrix: Mat4::one(),
      node,
    };
    inner
      .projection
      .update_projection(&mut inner.projection_matrix);

    inner.into()
  }

  pub fn resize(&self, size: (f32, f32)) {
    self.mutate(|mut camera| {
      let resize = CameraProjectionDelta::Resize(size);
      camera.modify(SceneCameraInnerDelta::projection(resize));

      let mut new_project = Mat4::one();
      camera.projection.update_projection(&mut new_project);
      camera.modify(SceneCameraInnerDelta::projection_matrix(new_project));
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

pub trait CameraProjection: Sync + Send + DynIncremental {
  fn update_projection(&self, projection: &mut Mat4<f32>);
  fn resize(&mut self, size: (f32, f32));
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32>;
  fn clone_self(&self) -> Box<dyn CameraProjection>;
}

impl<T> CameraProjection for T
where
  T: ResizableProjection<f32> + RayCaster3<f32> + DynIncremental + Clone + 'static,
{
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    self.update_projection::<WebGPU>(projection);
  }
  fn resize(&mut self, size: (f32, f32)) {
    self.resize(size);
  }
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32 {
    self.pixels_per_unit(distance, view_height)
  }

  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.cast_ray(normalized_position)
  }
  fn clone_self(&self) -> Box<dyn CameraProjection> {
    Box::new(self.clone())
  }
}

#[derive(Clone)]
pub enum CameraProjectionDelta {
  Resize((f32, f32)),
  Boxed(Box<dyn AnyClone>),
}

impl SimpleIncremental for Box<dyn CameraProjection> {
  type Delta = CameraProjectionDelta;

  fn s_apply(&mut self, delta: Self::Delta) {
    match delta {
      CameraProjectionDelta::Resize(size) => self.resize(size),
      CameraProjectionDelta::Boxed(delta) => self.as_mut().apply_dyn(delta).unwrap(),
    }
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    self
      .as_ref()
      .expand_dyn(&mut |d| cb(CameraProjectionDelta::Boxed(d)));
  }
}

#[derive(Incremental)]
pub struct SceneCameraInner {
  pub bounds: CameraViewBounds,
  pub projection: Box<dyn CameraProjection>,
  pub projection_matrix: Mat4<f32>,
  pub node: SceneNode,
}

impl AsRef<Self> for SceneCameraInner {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl SceneCameraInner {
  pub fn view_size_in_pixel(&self, frame_size: Size) -> Vec2<f32> {
    let width: usize = frame_size.width.into();
    let width = width as f32 * self.bounds.width;
    let height: usize = frame_size.height.into();
    let height = height as f32 * self.bounds.height;
    (width, height).into()
  }
}
