use crate::*;

pub type SceneIdUniformBufferAccess = LockReadGuardHolder<SceneIdUniforms>;
pub type SceneIdUniforms = UniformUpdateContainer<EntityHandle<SceneEntity>, Vec4<u32>>;

pub fn use_scene_id_provider(cx: &mut impl QueryGPUHookCx) -> Option<SceneIdUniformBufferAccess> {
  cx.use_uniform_buffers(|source, ctx| {
    source.with_source(
      global_watch()
        .watch_entity_set()
        .key_as_value()
        .collective_map(|v| v.into_raw().index())
        .into_query_update_uniform(0, ctx),
    )
  })
}

// pub type SceneIdUniformBufferAccess = LockReadGuardHolder<SceneIdUniforms>;
// pub type SceneIdUniforms = UniformBufferCollection<u32, Vec4<u32>>;

// pub fn use_scene_id_provider(cx: &mut impl QueryGPUHookCx) -> SceneIdUniformBufferAccess {
//   // let uniforms = cx.use_uniform_buffers2();

//   // cx.use_changes().map(|changes|{

//   // })
//   cx.use_uniform_buffers(|source, ctx| {
//     source.with_source(
//       global_watch()
//         .watch_entity_set()
//         .key_as_value()
//         .collective_map(|v| v.into_raw().index())
//         .into_query_update_uniform(0, ctx),
//     )
//   })
//   // uniforms.make_read_holder()
// }
