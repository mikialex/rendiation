mod directional;
pub use directional::*;
mod point;
pub use point::*;
mod spot;
pub use spot::*;

use crate::*;

#[derive(Clone)]
pub struct LightResourceCtx {
  pub shadow_system: Arc<RwLock<SingleProjectShadowMapSystem>>,
  pub derives: SceneNodeDeriveSystem,
}

pub trait WebGPULight: Any {
  type Uniform: Std140 + Any;
  // todo remove box
  fn create_uniform_stream(
    &self,
    ctx: &LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> Box<dyn Stream<Item = Self::Uniform> + Unpin>;
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

impl<T> WebGPUSceneLight for IncrementalSignalPtr<T>
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

#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

// note, we have to use the real name but not the ENode<ShaderAPIInstance> or we can not pass the
// rust orphan rules
impl core::ops::Add for ShaderLightingResultShaderAPIInstance {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      diffuse: self.diffuse + rhs.diffuse,
      specular: self.specular + rhs.specular,
    }
  }
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
  /// for given light type, this method will only called once in shader building process
  /// user could inject any custom dependency in shader that shared among all light instance
  fn create_dep(_builder: &mut ShaderFragmentBuilderView) {}

  fn compute_direct_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
  ) -> ENode<ShaderLightingResult>;
}

/// Punctual lights are defined as parameterized, infinitely small points that
/// emit light in well-defined directions and intensities.
pub trait PunctualShaderLight:
  ShaderStructuralNodeType + ShaderSizedValueNodeType + Std140 + Sized + Default
{
  /// see base trait similar method
  fn create_punctual_dep(_builder: &mut ShaderFragmentBuilderView) {}

  fn compute_incident_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight>;
}

impl<T: PunctualShaderLight> ShaderLight for T {
  fn create_dep(builder: &mut ShaderFragmentBuilderView) {
    T::create_punctual_dep(builder)
  }

  fn compute_direct_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
  ) -> ENode<ShaderLightingResult> {
    let incident = T::compute_incident_light(builder, light, ctx);

    incident
      .color
      .equals(Vec3::zero())
      .all()
      .select_branched(
        || {
          ShaderLightingResult::construct(ENode::<ShaderLightingResult> {
            diffuse: val(Vec3::zero()),
            specular: val(Vec3::zero()),
          })
        },
        || {
          shading_impl
            .compute_lighting_by_incident_dyn(shading, &incident, ctx)
            .construct()
        },
      )
      .expand()
  }
}
