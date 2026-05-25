use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightUniform {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: HighPrecisionTranslationUniform,
  pub cutoff_distance: f32,
}

pub type PerScenePointLightUniform = FastHashMap<
  RawEntityHandle,
  UniformBufferDataView<UniformArrayWithLengthInfo<PointLightUniform>>,
>;
pub type PointLightUniforms = (
  PerSceneLightUniformArray<PointLightUniform>,
  Arc<RwLock<PerScenePointLightUniform>>,
);

pub fn use_point_per_scene_uniform_array_buffers(
  cx: &mut QueryGPUHookCx,
) -> Option<PointLightUniforms> {
  // let changed = cx.use_db_entity_any_change::<SpotlightEntity>(); todo
  let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

  let uniform_array_caches = cx.use_shared_hash_map("point light uniform_array_caches");

  cx.when_render(|| {
    let world = world_mat.expect_resolve_stage();
    let r = create_point_light_uniform(&|node| world.access(&node).unwrap());

    sync_per_scene_uniforms(&r, &uniform_array_caches, &cx.gpu);

    (r, uniform_array_caches.clone())
  })
}

pub fn create_point_light_uniform(
  node_world_mat: &dyn Fn(RawEntityHandle) -> Mat4<f64>,
) -> PerSceneLightUniformArray<PointLightUniform> {
  let light_ref_scene = get_db_view::<PointLightRefScene>();
  let light_ref_node = get_db_view::<PointLightRefNode>();

  let intensity = get_db_view::<PointLightIntensity>();
  let cutoff = get_db_view::<PointLightCutOffDistance>();

  let iter_lights = light_ref_scene.iter_key_value().filter_map(|(light, s)| {
    let s = s?;
    let world_mat = node_world_mat(light_ref_node.access(&light)??);
    let position = into_hpt(world_mat.position()).into_uniform();
    let light_data = PointLightUniform {
      luminance_intensity: intensity.access(&light)?,
      cutoff_distance: cutoff.access(&light)?,
      position,
      ..Default::default()
    };

    (light, s, light_data).into()
  });

  compute_light_list(iter_lights)
}
