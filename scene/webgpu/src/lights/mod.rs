pub mod directional;
pub use directional::*;
use rendiation_algebra::*;
use shadergraph::*;

#[derive(Default)]
pub struct LightList<T> {
  pub lights: Vec<T>,
}

impl<T> LightList<T> {
  //
}

pub struct DrawDefer<'a, T> {
  pub light: &'a T,
}

// impl<'a, T: DirectShaderLight> ShaderGraphProvider for DrawDefer<'a, T> {
//   fn build(
//     &self,
//     builder: &mut ShaderGraphRenderPipelineBuilder,
//   ) -> Result<(), ShaderGraphBuildError> {
//     builder.fragment(|builder, binding| {
//       // let position = builder.query::<WorldFragmentPosition>()?.get();
//       // let normal = builder.query::<WorldFragmentNormal>()?.get();
//       // let camera_position = builder.query::<CameraPosition>()?.get();

//       // let view_dir = camera_position - position;

//       // let ctx = ExpandedNode::<ShaderLightingGeometricCtx> {
//       //   position,
//       //   normal,
//       //   view_dir,
//       // };

//       // let incident_light = T::compute_direct_light(todo!(), &ctx);

//       Ok(())
//     })
//   }
// }

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

pub trait ShaderLight: ShaderGraphStructuralNodeType + Sized {
  fn name() -> &'static str;
}

pub trait DirectShaderLight: ShaderLight {
  fn compute_direct_light(
    node: &ExpandedNode<Self>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}
