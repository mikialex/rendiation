use rendiation_algebra::Vec3;

pub trait DirectSceneLight {}

pub trait IndirectSceneLight {}

pub struct DirectionLight {
  direction: Vec3<f32>,
  intensity: Vec3<f32>,
}

pub struct AmbientLight {
  intensity: Vec3<f32>,
}
