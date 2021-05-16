use crate::NormalizedVec3;
use rendiation_algebra::*;

pub struct LightShapeSampleResult {
  pub local_position: Vec3<f32>,
  pub normal: NormalizedVec3<f32>,
  pub pdf: f32,
}

pub trait LightShape: Send + Sync {
  fn sample(&self) -> LightSampleResult;
}

pub struct Light {
  pub emissive: Vec3<f32>,
  pub shape: Box<dyn LightShape>,
}

pub struct LightSampleResult {
  pub emissive: Vec3<f32>,
  pub light_in_dir: NormalizedVec3<f32>,
}

impl Light {}

// pub trait Light: Sync + 'static {
//   fn sample<'a>(
//     &self,
//     world_position: Vec3<f32>,
//     scene: &RayTraceScene<'a>,
//     node: &SceneNode,
//   ) -> Option<LightSampleResult>;
// }

// pub trait LightToBoxed: Light + Sized {
//   fn to_boxed(self) -> Box<dyn Light> {
//     Box::new(self) as Box<dyn Light>
//   }
// }

// impl LightToBoxed for PointLight {}
// impl Light for PointLight {
//   fn sample<'a>(
//     &self,
//     world_position: Vec3<f32>,
//     scene: &RayTraceScene<'a>,
//     node: &SceneNode,
//   ) -> Option<LightSampleResult> {
//     let light_position = node.world_matrix.position();

//     if !scene.test_point_visible_to_point(light_position, world_position) {
//       return None;
//     }
//     let light_in_dir = world_position - light_position;
//     let distance = light_in_dir.length();
//     Some(LightSampleResult {
//       emissive: self.intensity / (distance * distance),
//       light_in_dir: light_in_dir.into_normalized(),
//     })
//   }
// }
