pub mod directional;
pub use directional::*;
pub mod point;
pub use point::*;
pub mod spot;
pub use spot::*;

use crate::*;

#[derive(Clone)]
pub struct LightResourceCtx {
  pub shadow_system: Arc<RwLock<SingleProjectShadowMapSystem>>,
  pub derives: SceneNodeDeriveSystem,
}

pub trait WebGPULight: Any {
  type Uniform: Std140 + Any;
  fn create_uniform_stream(
    &self,
    ctx: &LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> impl Stream<Item = Self::Uniform> + Unpin;
}

pub trait DynamicLightUniform: Any {
  fn as_bytes(&self) -> &[u8];
}

impl<T: Std140 + Any> DynamicLightUniform for T {
  fn as_bytes(&self) -> &[u8] {
    self.as_bytes()
  }
}

pub trait WebGPUSceneLight {
  fn create_uniform(
    &self,
    ctx: &mut LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> Box<dyn Stream<Item = Box<dyn DynamicLightUniform>>>;
}

impl<T> WebGPUSceneLight for SceneItemRef<T>
where
  Self: WebGPULight,
  T: IncrementalBase,
{
  fn create_uniform(
    &self,
    ctx: &mut LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> Box<dyn Stream<Item = Box<dyn DynamicLightUniform>>> {
    Box::new(
      self
        .create_uniform_stream(ctx, node)
        .map(|uni| Box::new(uni) as Box<dyn DynamicLightUniform>),
    )
  }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  /// from light source to surface
  pub direction: Vec3<f32>,
}

only_fragment!(HDRLightResult, Vec3<f32>);
only_fragment!(LDRLightResult, Vec3<f32>);

#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingGeometricCtx {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  /// from surface to the camera
  pub view_dir: Vec3<f32>,
}

pub trait ShaderLight:
  ShaderStructuralNodeType + ShaderSizedValueNodeType + Std140 + Sized + Default
{
  type Dependency;

  fn create_dep(builder: &mut ShaderFragmentBuilderView) -> Self::Dependency;

  fn compute_direct_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    dep: &Self::Dependency,
  ) -> ENode<ShaderLightingResult>;
}

/// Punctual lights are defined as parameterized, infinitely small points that
/// emit light in well-defined directions and intensities.
pub trait PunctualShaderLight:
  ShaderStructuralNodeType + ShaderSizedValueNodeType + Std140 + Sized + Default
{
  type PunctualDependency;

  fn create_punctual_dep(builder: &mut ShaderFragmentBuilderView) -> Self::PunctualDependency;

  fn compute_incident_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    dep: &Self::PunctualDependency,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight>;
}

impl<T: PunctualShaderLight> ShaderLight for T {
  type Dependency = T::PunctualDependency;

  fn create_dep(builder: &mut ShaderFragmentBuilderView) -> Self::Dependency {
    T::create_punctual_dep(builder)
  }

  fn compute_direct_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    dep: &Self::Dependency,
  ) -> ENode<ShaderLightingResult> {
    // todo, check if incident light intensity zero
    let incident = T::compute_incident_light(builder, light, dep, ctx);
    shading_impl.compute_lighting_by_incident_dyn(shading, &incident, ctx)
  }
}
