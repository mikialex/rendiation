use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub shadow: LightShadowAddressInfo,
}

impl PunctualShaderLight for DirectionalLightShaderInfo {
  type PunctualDependency = ();

  fn create_punctual_dep(
    _: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(())
  }

  fn compute_incident_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    _dep: &Self::PunctualDependency,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let shadow_factor =
      compute_occlusion_basic(builder, light.shadow, directional_shadow_occlusion);

    ENode::<ShaderIncidentLight> {
      color: light.intensity * shadow_factor,
      direction: light.direction,
    }
  }
}

wgsl_fn!(
  /// custom extra culling for directional light
  fn directional_shadow_occlusion(
    shadow_position: vec3<f32>,
    map: texture_depth_2d_array,
    d_sampler: sampler_comparison,
    info: ShadowMapAddressInfo,
  ) -> f32 {

    // maybe we could use sampler's border color config, but that's not part of standard webgpu (wgpu supports)
    let inFrustumVec = vec4<bool>(shadow_position.x >= 0.0, shadow_position.x <= 1.0, shadow_position.y >= 0.0, shadow_position.y <= 1.0);
    let inFrustum = all(inFrustumVec);
    let frustumTestVec = vec2<bool>(inFrustum, shadow_position.z <= 1.0);
    let frustumTest = all(frustumTestVec);

    if (frustumTest) {
      return textureSampleCompareLevel(
        map,
        d_sampler,
        shadow_position.xy,
        info.layer_index,
        shadow_position.z
      );
    } else {
      return 1.0;
    }
  }
);

impl WebGPUSceneLight for SceneLight<DirectionalLight> {
  // allocate shadow maps
  fn pre_update(&self, ctx: &mut LightUpdateCtx) {
    let inner = self.read();
    request_basic_shadow_map(&inner, ctx.ctx.resources, ctx.shadows);
  }

  fn update(&self, ctx: &mut LightUpdateCtx) {
    let inner = self.read();
    let light = &inner.light;
    let node = &inner.node;

    let shadow = check_update_basic_shadow_map(&inner, ctx);

    let lights = ctx.forward.get_or_create_list();
    let gpu = DirectionalLightShaderInfo {
      intensity: light.intensity,
      direction: node.get_world_matrix().forward().normalize().reverse(),
      shadow,
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}

pub struct DirectionalShadowMapExtraInfo {
  pub width_extend: f32,
  pub height_extend: f32,
  pub up: Vec3<f32>,
  pub enable_shadow: bool,
}

impl ShadowCameraCreator for SceneLightInner<DirectionalLight> {
  fn build_shadow_camera(&self) -> SceneCamera {
    let orth = OrthographicProjection {
      left: -20.,
      right: 20.,
      top: 20.,
      bottom: -20.,
      near: 0.1,
      far: 2000.,
    };
    let orth = WorkAroundResizableOrth { orth };
    SceneCamera::create_camera(orth, self.node.clone())
  }
}

struct WorkAroundResizableOrth<T> {
  orth: OrthographicProjection<T>,
}

impl<T: Scalar> Projection<T> for WorkAroundResizableOrth<T> {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<T>) {
    self.orth.update_projection::<S>(projection);
  }

  fn pixels_per_unit(&self, distance: T, view_height: T) -> T {
    self.orth.pixels_per_unit(distance, view_height)
  }
}

impl<T: Scalar> ResizableProjection<T> for WorkAroundResizableOrth<T> {
  fn resize(&mut self, _size: (T, T)) {
    // nothing!
  }
}
impl HyperRayCaster<f32, Vec3<f32>, Vec2<f32>> for WorkAroundResizableOrth<f32> {
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> HyperRay<f32, Vec3<f32>> {
    self.orth.cast_ray(normalized_position)
  }
}
