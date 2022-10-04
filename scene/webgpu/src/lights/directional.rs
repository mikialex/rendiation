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
pub struct ShadowMapShader {
  pub shadow_infos: Node<Shader140Array<BasicShadowMapInfo, 8>>,
  pub map: Node<ShaderDepthTexture2DArray>,
}

impl PunctualShaderLight for DirectionalLightShaderInfo {
  type PunctualDependency = ShadowMapShader;

  fn create_punctual_dep(
    builder: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(ShadowMapShader {
      shadow_infos: builder.query::<BasicShadowMapInfoGroup>()?,
      map: builder.query::<BasicShadowMap>()?,
    })
  }

  fn compute_incident_light(
    light: &ENode<Self>,
    dep: &Self::PunctualDependency,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let shadow_info = light.shadow.expand();
    let occlusion = consts(0.).mutable();

    if_by(shadow_info.enabled, || {
      let shadow_info = dep.shadow_infos.index(shadow_info.index);
    });

    ENode::<ShaderIncidentLight> {
      color: light.intensity * (consts(1.) - occlusion.get()),
      direction: light.direction,
    }
  }
}

impl WebGPUSceneLight for DirectionalLight {
  fn collect(&self, sys: &mut ForwardLightingSystem, node: &SceneNode) {
    let lights = sys.get_or_create_list();

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

struct DirectionalShadowMapExtraInfo {
  width_extend: f32,
  height_extend: f32,
  up: Vec3<f32>,
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
