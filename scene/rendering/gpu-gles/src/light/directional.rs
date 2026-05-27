use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightUniform {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub follow_camera: Bool,
}

pub fn use_directional_per_scene_uniform_array_buffers(
  cx: &mut QueryGPUHookCx,
) -> Option<SharedLightUniformInfo<DirectionalLightUniform>> {
  let uniform_array_caches = use_shared_light_uniform_info(cx, "directional");

  cx.skip_if_not_waked(|cx| {
    cx.use_db_entity_any_change::<DirectionalLightEntity>();
    let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

    if cx.is_in_render() {
      let world = world_mat.expect_resolve_stage();
      let r = create_directional_light_uniform(&|node| world.access(&node).unwrap().into_f32());

      sync_per_scene_uniforms(&r, &uniform_array_caches, &cx.gpu);
    }
  });

  cx.when_render(|| uniform_array_caches.clone())
}

pub fn create_directional_light_uniform(
  node_world_mat: &dyn Fn(RawEntityHandle) -> Mat4<f32>,
) -> PerSceneLightUniformArray<DirectionalLightUniform> {
  let light_ref_scene = get_db_view::<DirectionalRefScene>();
  let light_ref_node = get_db_view::<DirectionalRefNode>();

  let ill = get_db_view::<DirectionalLightIlluminance>();
  let follow_camera = get_db_view::<DirectionalLightFollowCamera>();

  let iter_lights = light_ref_scene.iter_key_value().filter_map(|(light, s)| {
    let s = s?;
    let world_mat = node_world_mat(light_ref_node.access(&light)??);
    let direction = world_mat.forward().reverse().normalize();
    let light_data = DirectionalLightUniform {
      illuminance: ill.access(&light)?,
      follow_camera: follow_camera.access(&light)?.into(),
      direction,
      ..Default::default()
    };

    (light, s, light_data).into()
  });

  compute_light_list(iter_lights)
}
