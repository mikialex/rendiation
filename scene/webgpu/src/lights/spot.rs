use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightShaderInfo {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
  pub shadow: LightShadowAddressInfo,
}

impl PunctualShaderLight for SpotLightShaderInfo {
  fn compute_incident_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - light.position;
    let distance = direction.length();
    let distance_factor =
      punctual_light_intensity_to_illuminance_factor_fn(distance, light.cutoff_distance);

    let direction = direction.normalize();
    let angle_cos = direction.dot(light.direction);
    let angle_factor = angle_cos.smoothstep(light.half_cone_cos, light.half_penumbra_cos);

    let shadow_info = light.shadow.expand();
    let occlusion = val(1.).make_local_var();

    let intensity_factor = distance_factor * angle_factor;

    if_by_ok(shadow_info.enabled.equals(1), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).load().expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      // we should have kept all light effective places inside the shadow volume
      if_by(intensity_factor.greater_than(0.), || {
        occlusion.store(sample_shadow(
          shadow_position,
          map,
          sampler,
          shadow_info.map_info,
        ))
      });
      Ok(())
    })
    .unwrap();

    let shadow_factor = val(1.) - occlusion.load();

    ENode::<ShaderIncidentLight> {
      color: light.luminance_intensity * intensity_factor * shadow_factor,
      direction,
    }
  }
}

impl WebGPULight for IncrementalSignalPtr<SpotLight> {
  type Uniform = SpotLightShaderInfo;

  fn create_uniform_stream(
    &self,
    ctx: &LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> Box<dyn Stream<Item = Self::Uniform> + Unpin> {
    enum ShaderInfoDelta {
      DirPosition(Vec3<f32>, Vec3<f32>),
      Shadow(LightShadowAddressInfo),
      Light(SpotLightShaderInfoPart),
    }

    struct SpotLightShaderInfoPart {
      pub luminance_intensity: Vec3<f32>,
      pub cutoff_distance: f32,
      pub half_cone_cos: f32,
      pub half_penumbra_cos: f32,
    }

    let node = node.create_broad_caster();
    let derives = ctx.derives.clone();
    let direction = node
      .fork_stream()
      .filter_map_sync(move |node| derives.create_world_matrix_stream(&node))
      .flatten_signal()
      .map(|mat| (mat.forward().reverse().normalize(), mat.position()))
      .map(|(a, b)| ShaderInfoDelta::DirPosition(a, b));

    let shadow = ctx
      .shadow_system
      .write()
      .unwrap()
      .create_shadow_info_stream(
        self.guid(),
        build_shadow_projection(self),
        node.fork_stream(),
      )
      .map(ShaderInfoDelta::Shadow);

    let light = self
      .single_listen_by(any_change)
      .filter_map_sync(self.defer_weak())
      .map(|light| {
        let light = light.read();
        SpotLightShaderInfoPart {
          luminance_intensity: light.luminance_intensity * light.color_factor,
          cutoff_distance: light.cutoff_distance,
          half_cone_cos: light.half_cone_angle.cos(),
          half_penumbra_cos: light.half_penumbra_angle.cos(),
        }
      })
      .map(ShaderInfoDelta::Light);

    let delta = futures::stream_select!(direction, shadow, light);

    Box::new(
      delta.fold_signal(SpotLightShaderInfo::default(), |delta, info| {
        match delta {
          ShaderInfoDelta::DirPosition(dir, pos) => {
            info.direction = dir;
            info.position = pos;
          }
          ShaderInfoDelta::Shadow(shadow) => info.shadow = shadow,
          ShaderInfoDelta::Light(l) => {
            info.luminance_intensity = l.luminance_intensity;
            info.cutoff_distance = l.cutoff_distance;
            info.half_penumbra_cos = l.half_penumbra_cos;
            info.half_cone_cos = l.half_cone_cos;
          }
        };
        Some(*info)
      }),
    )
  }
}

fn build_shadow_projection(
  light: &IncrementalSignalPtr<SpotLight>,
) -> impl Stream<Item = (CameraProjectionEnum, Size)> {
  light
    .single_listen_by(any_change)
    .filter_map_sync(light.defer_weak())
    .map(|light| {
      let proj = PerspectiveProjection {
        near: 0.1,
        far: 2000.,
        fov: Deg::from_rad(light.read().half_cone_angle * 2.),
        aspect: 1.,
      };
      let proj = CameraProjectionEnum::Perspective(proj);
      let size = Size::from_u32_pair_min_one((512, 512));
      (proj, size)
    })
}
