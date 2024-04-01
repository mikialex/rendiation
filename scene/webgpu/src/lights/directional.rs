use rendiation_geometry::{HyperRayCaster, Ray3};

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
  fn compute_incident_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let shadow_info = light.shadow.expand();
    let occlusion = val(1.).make_local_var();

    if_by_ok(shadow_info.enabled.equals(1), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).load().expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      if_by(cull_directional_shadow(shadow_position), || {
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

    ENode::<ShaderIncidentLight> {
      color: light.illuminance * (val(1.) - occlusion.load()),
      direction: light.direction,
    }
  }
}

/// custom extra culling for directional light
fn cull_directional_shadow(shadow_position: Node<Vec3<f32>>) -> Node<bool> {
  let left = shadow_position.x().greater_equal_than(val(0.));
  let right = shadow_position.x().less_equal_than(val(1.));
  let top = shadow_position.y().greater_equal_than(val(0.));
  let bottom = shadow_position.y().less_equal_than(val(1.));
  let far = shadow_position.z().less_equal_than(val(1.));

  left.and(right).and(top).and(bottom).and(far)
}

impl WebGPULight for IncrementalSignalPtr<DirectionalLight> {
  type Uniform = DirectionalLightShaderInfo;

  fn create_uniform_stream(
    &self,
    ctx: &LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> Box<dyn Stream<Item = Self::Uniform> + Unpin> {
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
      .shadow_system
      .write()
      .unwrap()
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

    Box::new(
      delta.fold_signal(DirectionalLightShaderInfo::default(), |delta, info| {
        match delta {
          ShaderInfoDelta::Dir(dir) => info.direction = dir,
          ShaderInfoDelta::Shadow(shadow) => info.shadow = shadow,
          ShaderInfoDelta::Ill(i) => info.illuminance = i,
        };
        Some(*info)
      }),
    )
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
  light: &IncrementalSignalPtr<DirectionalLight>,
) -> impl Stream<Item = (CameraProjectionEnum, Size)> {
  get_dyn_trait_downcaster_static!(CameraProjection).register::<WorkAroundResizableOrth>();
  light
    .single_listen_by(any_change)
    .filter_map_sync(light.defer_weak())
    .map(|_| {
      let shadow_info = DirectionalShadowMapExtraInfo::default();
      let size = Size::from_u32_pair_min_one((512, 512)); // todo
      let orth = WorkAroundResizableOrth {
        orth: shadow_info.range,
      };
      let proj = CameraProjectionEnum::Foreign(Box::new(orth));
      (proj, size)
    })
}

#[derive(Clone, PartialEq)]
struct WorkAroundResizableOrth {
  orth: OrthographicProjection<f32>,
}
clone_self_diffable_incremental!(WorkAroundResizableOrth);
type_as_dyn_trait!(WorkAroundResizableOrth, CameraProjection);

impl CameraProjection for WorkAroundResizableOrth {
  fn compute_projection_mat(&self) -> Mat4<f32> {
    self.orth.compute_projection_mat::<WebGPU>()
  }

  fn resize(&mut self, _: (f32, f32)) {
    // nothing!
  }

  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32 {
    self.orth.pixels_per_unit(distance, view_height)
  }

  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.orth.cast_ray(normalized_position)
  }
}
