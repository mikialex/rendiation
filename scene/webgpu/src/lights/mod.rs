pub mod directional;
pub use directional::*;

use crate::*;

pub struct LightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: UniformBufferDataView<Shader140Array<T, 32>>,
}

impl<T: ShaderLight> LightList<T> {
  pub fn update(&mut self) {
    //
  }

  // pub fn collect_lights_for_naive_forward<S: LightableSurfaceShading>(
  //   builder: &mut ShaderGraphRenderPipelineBuilder,
  //   shading: ExpandedNode<S>,
  // ) -> Result<(), ShaderGraphBuildError> {
  //   builder.fragment(|builder, binding| {
  //     let lights = todo!();
  //     let light_result = todo!();
  //     // for_by(lights, |_, _| {
  //     //   //
  //     // });
  //     Ok(())
  //   })
  // }
}

pub trait LightCollection {
  // fn collect_lights_for_naive_forward(
  //   &self,
  //   builder: &mut ShaderGraphRenderPipelineBuilder,
  // ) -> Result<(), ShaderGraphBuildError>;
  fn for_each_lights(&self, visitor: &dyn Fn(&dyn Any));
}

pub struct LightSystem {
  pub lights: Vec<Box<dyn LightCollection>>,
}

impl LightSystem {
  // pub fn collect_lights_for_naive_forward<S: LightableSurfaceShading>(
  //   &self,
  //   builder: &mut ShaderGraphRenderPipelineBuilder,
  // ) -> Result<(), ShaderGraphBuildError> {
  //   for light in &self.lights {
  //     light.collect_lights_for_naive_forward(builder)?
  //   }
  //   Ok(())
  // }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  pub direction: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingGeometricCtx {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub view_dir: Vec3<f32>,
}

pub trait ShaderLight: ShaderGraphStructuralNodeType + Std140 + Sized {
  fn name() -> &'static str;
}

pub trait DirectShaderLight: ShaderLight {
  fn compute_direct_light(
    node: &ExpandedNode<Self>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}
