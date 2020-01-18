use crate::transformed_object::TransformedObject;
use crate::controller::Controller;
use rendiation_math::Vec3;
use rendiation_math_entity::Spherical;

struct OrbitController {
  spherical: Spherical,

  rotateAngleFactor: f32,
  panFactor: f32,
  zoomFactor: f32,

  // restriction
  maxPolarAngle: f32,
  minPolarAngle: f32,

  // damping
  sphericalDelta: Spherical,
  zooming: f32,
  panOffset: Vec3<f32>,

  enableDamping: bool,
  zoomingDampingFactor: f32,
  rotateDampingFactor: f32,
  panDampingFactor: f32,

  viewWidth: f32,
  viewHeight: f32,
}

impl OrbitController {
  pub fn new() -> Self {
    Self {
      spherical: Spherical::new(),

      rotateAngleFactor: 0.2,
      panFactor: 0.0002,
      zoomFactor: 0.3,

      // restriction
      maxPolarAngle: 179. / 180. * std::f32::consts::PI,
      minPolarAngle: 0.1,

      // damping
      sphericalDelta: Spherical::new(),
      zooming: 1.0,
      panOffset: Vec3::new(0.0, 0.0, 0.0),

      enableDamping: true,
      zoomingDampingFactor: 0.1,
      rotateDampingFactor: 0.1,
      panDampingFactor: 0.1,

      viewWidth: 1000.,
      viewHeight: 1000.,
    }
  }
}

impl<T: TransformedObject> Controller<T> for OrbitController {
  fn update(&self, object: T) {

  }
}