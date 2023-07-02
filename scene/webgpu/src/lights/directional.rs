use rendiation_geometry::{HyperRay, HyperRayCaster, Ray3};

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

impl WebGPULight for SceneItemRef<DirectionalLight> {
  type Uniform = DirectionalLightShaderInfo;

  fn create_uniform_stream(
    &self,
    ctx: &mut LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> impl Stream<Item = Self::Uniform> {
    enum ShaderInfoDelta {
      Dir(Vec3<f32>),
      Shadow(LightShadowAddressInfo),
      Ill(Vec3<f32>),
    }

    let node = node.create_broad_caster();
    let derives = ctx.derives.clone();
    let direction = node
      .fork_stream()
      .filter_map_sync(move |node| derives.create_world_matrix_stream(&node))
      .flatten_signal()
      .map(|mat| mat.forward().reverse().normalize())
      .map(ShaderInfoDelta::Dir);

    let shadow = ctx
      .shadow_system()
      .create_shadow_info_stream(
        self.guid(),
        build_shadow_projection(self),
        node.fork_stream(),
      )
      .map(ShaderInfoDelta::Shadow);

    let ill = self
      .single_listen_by(any_change)
      .filter_map_sync(self.defer_weak())
      .map(|light| light.read().illuminance * light.read().color_factor)
      .map(ShaderInfoDelta::Ill);

    let delta = futures::stream_select!(direction, shadow, ill);

    delta.fold_signal(DirectionalLightShaderInfo::default(), |delta, info| {
      match delta {
        ShaderInfoDelta::Dir(dir) => info.direction = dir,
        ShaderInfoDelta::Shadow(shadow) => info.shadow = shadow,
        ShaderInfoDelta::Ill(i) => info.illuminance = i,
      };
      Some(*info)
    })
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

fn build_shadow_projection(
  light: &SceneItemRef<DirectionalLight>,
) -> impl Stream<Item = (CameraProjector, Size)> {
  light
    .single_listen_by(any_change)
    .filter_map_sync(light.defer_weak())
    .map(|light| {
      let light = light.read();
      let shadow_info = light.ext.get::<DirectionalShadowMapExtraInfo>().unwrap(); // todo
      let size = Size::from_u32_pair_min_one((512, 512)); // todo
      let orth = WorkAroundResizableOrth {
        orth: shadow_info.range,
      };
      let proj = CameraProjector::Foreign(Arc::new(orth));
      (proj, size)
    })
}

#[derive(Clone)]
struct WorkAroundResizableOrth {
  orth: OrthographicProjection<f32>,
}
clone_self_diffable_incremental!(WorkAroundResizableOrth);

impl CameraProjection for WorkAroundResizableOrth {
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    self.orth.update_projection::<WebGPU>(projection);
  }

  fn resize(&mut self, size: (f32, f32)) {
    // nothing!
  }

  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32 {
    self.orth.pixels_per_unit(distance, view_height)
  }

  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.orth.cast_ray(normalized_position)
  }
}
