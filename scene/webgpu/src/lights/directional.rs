use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub shadow: LightShadowAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LightShadowAddressInfo {
  pub index: u32,
  pub enabled: Bool,
}

only_fragment!(BasicShadowMapInfoGroup, Shader140Array<BasicShadowMapInfo, 8>);
only_fragment!(BasicShadowMap, ShaderDepthTexture2DArray);
only_fragment!(BasicShadowMapSampler, ShaderCompareSampler);
only_fragment!(ShadowPosition, Vec3<f32>);

wgsl_fn!(
  fn directional_shadow_occlusion(
    shadow_position: vec3<f32>,
    map: texture_depth_2d_array,
    d_sampler: comparison_sampler,
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
        shadow_position.z,
      );
    } else {
      return 1.0;
    }
  }
);

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
    let shadow_info = light.shadow.expand();
    let occlusion = consts(0.).mutable();

    if_by(shadow_info.enabled, || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();
      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_position = builder.query::<ShadowPosition>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index);

      let occlusion_result =
        directional_shadow_occlusion(shadow_position, map, sampler, shadow_info.expand().map_info);

      occlusion.set(occlusion_result)
    });

    ENode::<ShaderIncidentLight> {
      color: light.intensity * (consts(1.) - occlusion.get()),
      direction: light.direction,
    }
  }
}

impl WebGPUSceneLight for DirectionalLight {
  fn update(&self, ctx: &mut LightUpdateCtx, node: &SceneNode) {
    let lights = ctx.forward.get_or_create_list();

    let gpu = DirectionalLightShaderInfo {
      intensity: self.intensity,
      direction: node.get_world_matrix().forward().normalize().reverse(),
      shadow: LightShadowAddressInfo {
        enabled: false.into(),
        ..Zeroable::zeroed()
      },
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}

pub struct DirectionalShadowMapExtraInfo {
  pub width_extend: f32,
  pub height_extend: f32,
  pub up: Vec3<f32>,
}

fn build_shadow_camera(light: &DirectionalLight, node: &SceneNode) -> CameraGPUTransform {
  let world = node.get_world_matrix();
  let eye = world.position();
  let front = eye + world.forward();
  let camera_world = Mat4::lookat(eye, front, Vec3::new(0., 1., 0.));

  let orth = OrthographicProjection {
    left: -20.,
    right: 20.,
    top: 20.,
    bottom: -20.,
    near: 0.1,
    far: 2000.,
  };

  let proj = orth.create_projection::<WebGPU>();
  CameraGPUTransform::from_proj_and_world(proj, world)
}
