use rendiation_texture_packer::{
  pack_2d_to_2d::pack_impl::etagere_wrap::EtagerePacker, pack_2d_to_3d::MultiLayerTexturePacker,
  RePackablePacker, TexturePackerInit,
};

use crate::*;

const CASCADE_SHADOW_SPLIT_COUNT: usize = 4;

/// As the cascade shadow map is highly dynamic(the change related to camera) and
/// the shadow count should be small, the implementation is none-incremental
pub struct CascadeShadowMapSystemInputs {
  /// alloc_id => shadow map world
  pub source_world: BoxedDynQuery<u32, Mat4<f64>>,
  /// alloc_id => shadow map proj
  pub source_proj: BoxedDynQuery<u32, Mat4<f32>>,
  /// alloc_id => shadow map resolution
  pub size: BoxedDynQuery<u32, Size>,
  /// alloc_id => shadow map bias
  pub bias: BoxedDynQuery<u32, ShadowBias>,
  /// alloc_id => enabled
  pub enabled: BoxedDynQuery<u32, bool>,
}

pub type CascadeShadowPackerImpl = MultiLayerTexturePacker<EtagerePacker>;

pub fn generate_cascade_shadow_info(
  inputs: CascadeShadowMapSystemInputs,
  packer_size: SizeWithDepth,
  view_camera_proj: Mat4<f32>,
  view_camera_world: Mat4<f64>,
  ndc: &dyn NDCSpaceMapper<f32>,
) -> (CascadeShadowPackerImpl, Vec<CascadeShadowMapInfo>) {
  let mut packer = CascadeShadowPackerImpl::init_by_config(packer_size);

  let mut uniforms: Vec<CascadeShadowMapInfo> = Vec::new();

  for (k, enabled) in inputs.enabled.iter_key_value() {
    if !enabled {
      continue;
    }

    let world_to_light = inputs.source_proj.access(&k).unwrap().into_f64()
      * inputs
        .source_world
        .access(&k)
        .unwrap()
        .inverse_or_identity();

    let cascades =
      compute_directional_light_cascade_info(view_camera_world, view_camera_proj, world_to_light);

    let light_world = inputs.source_world.access(&k).unwrap();
    let light_world_inv = light_world.inverse_or_identity();

    let mut cascade_info = Vec::<SingleShadowMapInfo>::with_capacity(CASCADE_SHADOW_SPLIT_COUNT);
    let size = inputs.size.access(&k).unwrap();

    for (sub_proj, _) in cascades {
      // todo, use none id pack method
      if let Ok(pack) = packer.pack_with_id(size) {
        let shadow_center_to_shadowmap_ndc_without_translation =
          sub_proj.compute_projection_mat(ndc) * light_world_inv.remove_position().into_f32();

        cascade_info.push(SingleShadowMapInfo {
          map_info: convert_pack_result(pack.result),
          shadow_center_to_shadowmap_ndc_without_translation,
          ..Default::default()
        });
      } else {
        return (packer, uniforms);
      }
    }

    let shadow_world_position = into_hpt(light_world.position()).into_uniform();

    uniforms.push(CascadeShadowMapInfo {
      bias: inputs.bias.access(&k).unwrap(),
      shadow_world_position,
      map_info: Shader140Array::from_slice_clamp_or_default(&cascade_info),
      ..Default::default()
    })
  }

  (packer, uniforms)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct CascadeShadowMapInfo {
  pub bias: ShadowBias,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub map_info: Shader140Array<SingleShadowMapInfo, CASCADE_SHADOW_SPLIT_COUNT>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct SingleShadowMapInfo {
  pub map_info: ShadowMapAddressInfo,
  pub shadow_center_to_shadowmap_ndc_without_translation: Mat4<f32>,
  pub split_distance: f32,
}

/// return per sub frustum light shadow camera projection mat and split distance
pub fn compute_directional_light_cascade_info(
  camera_world: Mat4<f64>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f64>,
) -> [(OrthographicProjection<f32>, f32); CASCADE_SHADOW_SPLIT_COUNT] {
  let (near, far) = camera_projection.get_near_far_assume_orthographic();
  compute_light_cascade_info(camera_world, camera_projection, world_to_light).map(
    |(min, max, split)| {
      let proj = OrthographicProjection {
        left: min.x,
        right: max.x,
        top: max.y,
        bottom: min.y,
        near,
        far,
      };
      (proj, split)
    },
  )
}

/// return per sub frustum min max point and split distance in light space
pub fn compute_light_cascade_info(
  camera_world: Mat4<f64>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f64>,
) -> [(Vec3<f32>, Vec3<f32>, f32); CASCADE_SHADOW_SPLIT_COUNT] {
  let (near, far) = camera_projection.get_near_far_assume_is_common_projection();

  let world_to_clip = camera_projection.into_f64() * camera_world;
  let clip_to_world = world_to_clip.inverse_or_identity();
  let frustum_corners = [
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(1.0, -1.0, 0.0),
    Vec3::new(-1.0, -1.0, 0.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(-1.0, -1.0, 1.0),
  ]
  .map(|v| clip_to_world * v);

  let ratio = ((far * far) / 1_000_000.0).min(1.0);
  let target_cascade_splits: [f32; CASCADE_SHADOW_SPLIT_COUNT] = std::array::from_fn(|i| {
    let p = (i as f32 + 1.0) / (CASCADE_SHADOW_SPLIT_COUNT as f32);
    let log = near.powf(1.0 - p) * far.powf(p);
    let linear = near + p * (far - near);
    linear.lerp(log, ratio)
  });

  let mut idx = 0;
  target_cascade_splits.map(|split_distance| {
    let far_distance = split_distance;
    let near_distance = if idx == 0 {
      near
    } else {
      target_cascade_splits[idx - 1]
    };

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for idx in 0..8 {
      let distance = if idx < 4 {
        // near plane
        near_distance
      } else {
        far_distance
      };

      let ratio = (distance - near) / (far - near);
      let corner_pair = (frustum_corners[idx % 4], frustum_corners[idx % 4 + 4]);
      let corner_position = corner_pair.0.lerp(corner_pair.1, ratio.into());

      let corner_position_in_light = world_to_light * corner_position;

      min = min.min(corner_position_in_light.into_f32());
      max = max.max(corner_position_in_light.into_f32());
    }
    idx += 1;
    (min, max, split_distance)
  })
}

/// compute the current shading point in which sub frustum
#[shader_fn]
pub fn compute_cascade_index(
  render_position: Node<Vec3<f32>>,
  camera_world_mat: Node<Mat4<f32>>,
  splits: Node<Vec4<f32>>,
) -> Node<u32> {
  let camera_position = camera_world_mat.position();
  let camera_forward_dir = camera_world_mat.forward().normalize();

  let diff = render_position - camera_position;
  let distance = diff.dot(camera_forward_dir);

  let x = splits.x();
  let y = splits.y();
  let z = splits.z();

  let offset = val(0_u32).make_local_var();

  if_by(distance.less_than(x), || {
    offset.store(val(0_u32));
  })
  .else_if(distance.less_than(y), || {
    offset.store(val(1_u32));
  })
  .else_if(distance.less_than(z), || {
    offset.store(val(2_u32));
  })
  .else_by(|| {
    offset.store(val(3_u32));
  });

  offset.load()
}
