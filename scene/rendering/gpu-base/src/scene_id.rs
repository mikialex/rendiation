use crate::*;

#[derive(Default)]
pub struct SceneIdProvider {
  token: QueryToken,
}
pub type SceneIdUniformBufferAccess = LockReadGuardHolder<SceneIdUniforms>;
pub type SceneIdUniforms = UniformUpdateContainer<EntityHandle<SceneEntity>, Vec4<u32>>;

impl QueryBasedFeature<SceneIdUniformBufferAccess> for SceneIdProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, ctx: &Self::Context) {
    let source = global_watch()
      .watch_entity_set()
      .key_as_value()
      .collective_map(|v| v.into_raw().index())
      .into_query_update_uniform(0, ctx);

    let uniforms = SceneIdUniforms::default().with_source(source);

    self.token = qcx.register_multi_updater(uniforms);
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.token);
  }
  fn create_impl(&self, cx: &mut QueryResultCtx) -> SceneIdUniformBufferAccess {
    cx.take_multi_updater_updated(self.token).unwrap()
  }
}
