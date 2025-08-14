use crate::*;

pub type SceneIdUniformBufferAccess = LockReadGuardHolder<SceneIdUniforms>;
pub type SceneIdUniforms = UniformBufferCollectionRaw<RawEntityHandle, Vec4<u32>>;

pub fn use_scene_id_provider(cx: &mut QueryGPUHookCx) -> SceneIdUniformBufferAccess {
  let uniforms = cx.use_uniform_buffers();

  cx.use_query_set::<SceneEntity>()
    .map(|v| {
      v.delta_key_as_value()
        .delta_map_value(|v| v.index())
        .into_change()
    })
    .update_uniforms(&uniforms, 0, cx.gpu);

  uniforms.make_read_holder()
}
