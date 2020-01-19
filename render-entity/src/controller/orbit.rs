use rendiation_math::Vec2;
use crate::transformed_object::TransformedObject;
use crate::controller::Controller;
use rendiation_math::*;
use rendiation_math_entity::Spherical;

pub struct OrbitController {
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

  needUpdate: bool,
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

      needUpdate: true,
    }
  }

  pub fn pan(&mut self, offset: Vec2<f32>) {
    // offset = offset * Mat2::rotate(Vec3::zero(), -self.spherical.azim);
    // offset.rotate(-self.spherical.azim);
    // offset *= self.spherical.radius * self.panFactor;
    self.panOffset.x += offset.x;
    self.panOffset.z += offset.y;
    self.needUpdate = true;
  }

  pub fn zoom(&mut self, factor: f32) {
    self.zooming = 1. + (factor - 1.) * self.zoomFactor;
    self.needUpdate = true;
  }

  pub fn rotate(&mut self, offset: Vec2<f32>) {
    self.sphericalDelta.polar += offset.y / self.viewHeight * std::f32::consts::PI * self.rotateAngleFactor;
    self.sphericalDelta.azim += offset.x / self.viewWidth * std::f32::consts::PI * self.rotateAngleFactor;
    self.needUpdate = true;
  }

}

impl<T: TransformedObject> Controller<T> for OrbitController {
  fn update(&mut self, target: &mut T) {
    if self.sphericalDelta.azim.abs() > 0.0001 ||
      self.sphericalDelta.polar.abs() > 0.0001 ||
      self.sphericalDelta.radius.abs() > 0.0001 ||
      (self.zooming - 1.).abs() > 0.0001 ||
      self.panOffset.length2() > 0.000_000_1
     {
      self.needUpdate = true;
    }


    // if self.needUpdate {
    //   self.spherical.radius *= self.zooming;

    //   self.spherical.azim += self.sphericalDelta.azim;

    //   self.spherical.polar = (self.spherical.polar + self.sphericalDelta.polar)
    //   .clamp(self.minPolarAngle, self.maxPolarAngle);

    //   // self.spherical.polar = MathUtil.clamp(
    //   //   self.spherical.polar + self.sphericalDelta.polar,
    //   //   self.minPolarAngle, self.maxPolarAngle);

    //   self.spherical.center += self.panOffset;

    //   tempVec.setFromSpherical(self.spherical).add(self.spherical.center);
    //   let transform = target.get_transform();
    //   transform.position.copy(tempVec);
    //   transform.lookAt(self.spherical.center, self.up);
    // }
    self.needUpdate = false;

    // update damping effect
    if self.enableDamping {
      self.sphericalDelta.azim *= 1. - self.rotateDampingFactor;
      self.sphericalDelta.polar *= 1. - self.rotateDampingFactor;
      self.zooming += (1. - self.zooming) * self.zoomingDampingFactor;
      self.panOffset *= 1. - self.panDampingFactor;
    } else {
      self.sphericalDelta.reset_pose();
      self.zooming = 1.;
      self.panOffset = Vec3::zero();
    }
  }
}