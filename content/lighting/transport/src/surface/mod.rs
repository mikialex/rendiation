use crate::*;

mod microfacet;
pub use microfacet::*;

pub struct ImportanceSampled<T, U> {
  pub sample: T,
  pub pdf: f32,
  pub importance: U,
}

pub type BRDFImportantSampled = ImportanceSampled<NormalizedVec3<f32>, Vec3<f32>>;

pub trait LightTransportSurface {
  fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> Vec3<f32>;

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32>;

  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> f32;

  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> BRDFImportantSampled {
    let light_dir = self.sample_light_dir_use_bsdf_importance_impl(view_dir, normal, sampler);
    ImportanceSampled {
      sample: light_dir,
      pdf: self.pdf(view_dir, light_dir, normal),
      importance: self.bsdf(view_dir, light_dir, normal),
    }
  }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingGeometricCtx {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  /// origin from surface to the camera
  pub view_dir: Vec3<f32>,
}

pub struct ShaderImportanceSampled<T, U> {
  pub sample: T,
  pub pdf: Node<f32>,
  pub importance: U,
}

pub type ShaderBRDFImportantSampled = ShaderImportanceSampled<Node<Vec3<f32>>, Node<Vec3<f32>>>;

/// https://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface#fragment-SamplerInterface-2
///
/// Because sample values must be strictly less than 1,
/// OneMinusEpsilon, that represents the largest representable floating-point constant that is less
/// than 1. Later, we will clamp sample vector values to be no larger than this value.
/// const ONE_MINUS_EPSILON: f32 = 0x1.ffffffep - 1;
///
/// The task of a Sampler is to generate a sequence of -dimensional samples in
/// [0, 1) ^ d
pub trait DeviceSampler {
  fn reset(&self, next_sampling_index: Node<u32>);

  fn next(&self) -> Node<f32>;

  /// While a 2D sample value could be constructed by using values returned by a pair of calls to
  /// sample(), some samplers can generate better point distributions if they know that two
  /// dimensions will be used together.
  fn next_2d(&self) -> Node<Vec2<f32>> {
    (self.next(), self.next()).into()
  }
}

pub trait ShaderLightTransportSurface {
  fn bsdf(&self, cx: ENode<ShaderLightingGeometricCtx>) -> Node<Vec3<f32>>;

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    cx: ENode<ShaderLightingGeometricCtx>,
    sampler: &mut dyn DeviceSampler,
  ) -> Node<Vec3<f32>>;

  fn pdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<f32>;

  fn sample_light_dir_use_bsdf_importance(
    &self,
    cx: ENode<ShaderLightingGeometricCtx>,
    sampler: &mut dyn DeviceSampler,
  ) -> ShaderBRDFImportantSampled {
    let light_dir = self.sample_light_dir_use_bsdf_importance_impl(cx, sampler);
    ShaderImportanceSampled {
      sample: light_dir,
      pdf: self.pdf(cx.view_dir, light_dir, cx.normal),
      importance: self.bsdf(cx),
    }
  }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  /// from light source to surface
  pub direction: Vec3<f32>,
}

pub trait LightableSurfaceShadingLogicProvider {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilder,
  ) -> Box<dyn LightableSurfaceShading>;
}

pub trait LightableSurfaceShading {
  fn compute_lighting_by_incident(
    &self,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult>;

  fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

impl core::ops::Add for ENode<ShaderLightingResult> {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      diffuse: self.diffuse + rhs.diffuse,
      specular: self.specular + rhs.specular,
    }
  }
}
