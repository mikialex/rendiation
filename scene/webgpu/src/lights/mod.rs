pub mod directional;
pub use directional::*;
pub mod point;
pub use point::*;
pub mod spot;
pub use spot::*;

use crate::*;

pub struct LightingCtx<'a, 'b> {
  pub forward: &'a mut ForwardLightingSystem,
  pub shadows: &'a mut ShadowMapSystem,
  pub scene: &'a SceneRenderResourceGroup<'a>,
  /// we need this to do the depth map pass encoding
  pub ctx: &'a mut FrameCtx<'b>,
}

// impl<'a, 'b> LightingCtx<'a, 'b> {
//   pub fn update(&mut self) {
//     self.forward.before_update_scene(self.ctx.gpu);
//     self.shadows.before_update_scene(self.ctx.gpu);

//     let lights = &self.scene.scene.lights;

//     for (_, light) in lights {
//       light.pre_update(self, &light.read().node)
//     }

//     for (_, light) in lights {
//       light.update(self, &light.read().node)
//     }
//     self.forward.after_update_scene(self.ctx.gpu);
//     self.shadows.after_update_scene(self.ctx.gpu);
//   }
// }

pub struct LightResourceCtx<'a> {
  pub providers: AnyMap,
  pub derives: &'a SceneNodeDeriveSystem,
}

impl<'a> LightResourceCtx<'a> {
  pub fn shadow_system(&self) {
    //
  }
}

pub trait WebGPULight {
  type Uniform: Std140 + Any;
  fn create_uniform_stream(
    &self,
    ctx: &mut LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode>>,
  ) -> impl Stream<Item = Self::Uniform>;
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
    node: Box<dyn Stream<Item = SceneNode>>,
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
    node: Box<dyn Stream<Item = SceneNode>>,
  ) -> Box<dyn Stream<Item = Box<dyn DynamicLightUniform>>> {
    Box::new(
      self
        .create_uniform_stream(ctx, node)
        .map(|uni| Box::new(uni) as Box<dyn DynamicLightUniform>),
    )
  }
}

// pub trait WebGPUSceneLight: Any {
//   fn pre_update(&self, _ctx: &mut LightUpdateCtx, _: &SceneNode) {}
//   fn update(&self, ctx: &mut LightUpdateCtx, node: &SceneNode);
// }
// define_dyn_trait_downcaster_static!(WebGPUSceneLight);
// pub fn register_webgpu_light_features<T>()
// where
//   T: AsRef<dyn WebGPUSceneLight> + AsMut<dyn WebGPUSceneLight> + 'static,
// {
//   get_dyn_trait_downcaster_static!(WebGPUSceneLight).register::<T>()
// }

// impl WebGPUSceneLight for SceneLight {
//   fn pre_update(&self, ctx: &mut LightingCtx, _: &SceneNode) {
//     let inner = self.read();
//     let light = &inner.light;
//     let node = &inner.node;

//     match light {
//       SceneLightKind::PointLight(l) => l.pre_update(ctx, node),
//       SceneLightKind::SpotLight(l) => l.pre_update(ctx, node),
//       SceneLightKind::DirectionalLight(l) => l.pre_update(ctx, node),
//       SceneLightKind::Foreign(l) => {
//         if let Some(l) =
// get_dyn_trait_downcaster_static!(WebGPUSceneLight).downcast_ref(l.as_ref())         {
//           l.pre_update(ctx, node);
//         }
//       }
//       _ => {}
//     }
//   }
//   fn update(&self, ctx: &mut LightingCtx, _: &SceneNode) {
//     let inner = self.read();
//     let light = &inner.light;
//     let node = &inner.node;

//     match light {
//       SceneLightKind::PointLight(l) => l.update(ctx, node),
//       SceneLightKind::SpotLight(l) => l.update(ctx, node),
//       SceneLightKind::DirectionalLight(l) => l.update(ctx, node),
//       SceneLightKind::Foreign(l) => {
//         if let Some(l) = l.downcast_ref::<Box<dyn WebGPUSceneLight>>() {
//           l.update(ctx, node);
//         }
//       }
//       _ => {}
//     }
//   }
// }

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  /// from light source to surface
  pub direction: Vec3<f32>,
}

only_fragment!(HDRLightResult, Vec3<f32>);
only_fragment!(LDRLightResult, Vec3<f32>);

#[derive(Copy, Clone, ShaderStruct)]
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
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized + Default
{
  /// this is to avoid mutable borrow errors in for_by and if_by.
  type Dependency;

  fn create_dep(
    builder: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::Dependency, ShaderGraphBuildError>;

  fn compute_direct_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    dep: &Self::Dependency,
  ) -> Result<ENode<ShaderLightingResult>, ShaderGraphBuildError>;
}

/// Punctual lights are defined as parameterized, infinitely small points that
/// emit light in well-defined directions and intensities.
pub trait PunctualShaderLight:
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized + Default
{
  type PunctualDependency;

  fn create_punctual_dep(
    builder: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError>;

  fn compute_incident_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    dep: &Self::PunctualDependency,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<ENode<ShaderIncidentLight>, ShaderGraphBuildError>;
}

impl<T: PunctualShaderLight> ShaderLight for T {
  type Dependency = T::PunctualDependency;

  fn create_dep(
    builder: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::Dependency, ShaderGraphBuildError> {
    T::create_punctual_dep(builder)
  }

  fn compute_direct_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    dep: &Self::Dependency,
  ) -> Result<ENode<ShaderLightingResult>, ShaderGraphBuildError> {
    // todo, check if incident light intensity zero
    let incident = T::compute_incident_light(builder, light, dep, ctx)?;
    shading_impl.compute_lighting_by_incident_dyn(shading, &incident, ctx)
  }
}
