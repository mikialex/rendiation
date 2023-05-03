use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  /// in lx
  pub illuminance: Vec3<f32>,
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
  ) -> Result<ENode<ShaderIncidentLight>, ShaderGraphBuildError> {
    let shadow_info = light.shadow.expand();
    let occlusion = consts(1.).mutable();

    if_by_ok(shadow_info.enabled.equals(consts(1)), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      if_by(cull_directional_shadow(shadow_position), || {
        occlusion.set(sample_shadow(
          shadow_position,
          map,
          sampler,
          shadow_info.map_info,
        ))
      });
      Ok(())
    })?;

    Ok(ENode::<ShaderIncidentLight> {
      color: light.illuminance * (consts(1.) - occlusion.get()),
      direction: light.direction,
    })
  }
}

wgsl_fn!(
  /// custom extra culling for directional light
  fn cull_directional_shadow(
    shadow_position: vec3<f32>,
  ) -> bool {
    // maybe we could use sampler's border color config, but that's not part of standard webgpu (wgpu supports)
    let inFrustumVec = vec4<bool>(shadow_position.x >= 0.0, shadow_position.x <= 1.0, shadow_position.y >= 0.0, shadow_position.y <= 1.0);
    let inFrustum = all(inFrustumVec);
    let frustumTestVec = vec2<bool>(inFrustum, shadow_position.z <= 1.0);
    return all(frustumTestVec);
  }
);

impl WebGPUSceneLight for SceneItemRef<DirectionalLight> {
  // allocate shadow maps
  fn pre_update(&self, ctx: &mut LightUpdateCtx, node: &SceneNode) {
    let inner = self.read();
    request_basic_shadow_map(&inner, ctx.ctx.scene_resources, ctx.shadows, node);
  }

  fn update(&self, ctx: &mut LightUpdateCtx, node: &SceneNode) {
    let light = self.read();
    let shadow = check_update_basic_shadow_map(&light, ctx, node);

    let lights = ctx.forward.get_or_create_list();
    let gpu = DirectionalLightShaderInfo {
      illuminance: light.illuminance * light.color_factor,
      direction: ctx
        .node_derives
        .get_world_matrix(node)
        .forward()
        .reverse()
        .normalize(),
      shadow,
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}

#[derive(Copy, Clone)]
pub struct DirectionalShadowMapExtraInfo {
  pub range: OrthographicProjection<f32>,
  // pub enable_shadow: bool,
}

impl Default for DirectionalShadowMapExtraInfo {
  fn default() -> Self {
    Self {
      range: OrthographicProjection {
        left: -20.,
        right: 20.,
        top: 20.,
        bottom: -20.,
        near: 0.1,
        far: 2000.,
      },
    }
  }
}

impl ShadowCameraCreator for DirectionalLight {
  fn build_shadow_camera(&self, node: &SceneNode) -> SceneCamera {
    let extra = self
      .ext
      .get::<DirectionalShadowMapExtraInfo>()
      .copied()
      .unwrap_or_default();

    let orth = WorkAroundResizableOrth { orth: extra.range };
    SceneCamera::new(orth, node.clone())
  }
}

#[derive(Clone)]
struct WorkAroundResizableOrth<T> {
  orth: OrthographicProjection<T>,
}

impl<T: Clone + Send + Sync> incremental::SimpleIncremental for WorkAroundResizableOrth<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta;
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

impl<T: Scalar> Projection<T> for WorkAroundResizableOrth<T> {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<T>) {
    self.orth.update_projection::<WebGPU>(projection);
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
