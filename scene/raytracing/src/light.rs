use dyn_clone::DynClone;
use rendiation_algebra::*;
use rendiation_geometry::SurfaceAreaMeasurable;

use crate::NormalizedVec3;

pub trait SurfaceAreaMeasureAble {
  fn surface_area(&self) -> f32;
}

impl<T: SurfaceAreaMeasurable<f32, Matrix = Mat4<f32>>> SurfaceAreaMeasureAble for T {
  fn surface_area(&self) -> f32 {
    self.surface_area()
  }
}

pub struct LightSourceShapeSample {
  pub position: Vec3<f32>,
  pub normal: NormalizedVec3<f32>,
}

/// https://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Sampling_Light_Sources#fragment-ShapeInterface-5
pub trait LightShape: Send + Sync + SurfaceAreaMeasureAble + DynClone {
  fn pdf(&self) -> f32 {
    1.0 / self.surface_area()
  }

  fn sample_on_light_source(&self) -> LightSourceShapeSample;
}

dyn_clone::clone_trait_object!(LightShape);

// impl LightShape for Sphere {
//   fn sample_on_light_source(&self) -> LightSourceShapeSample {
//     todo!()
//   }
// }

#[derive(Clone)]
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
