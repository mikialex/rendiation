use crate::*;

mod attribute;
pub use attribute::*;

pub trait IndirectModelShapeRenderImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      self.as_any().type_id().hash(hasher);
    })
  }

  fn as_any(&self) -> &dyn Any;

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    any_idx: EntityHandle<StandardModelEntity>,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<DrawCommandBuilder>;
}

impl IndirectModelShapeRenderImpl for Vec<Box<dyn IndirectModelShapeRenderImpl>> {
  fn make_component_indirect(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(com) = provider.make_component_indirect(idx) {
        return Some(com);
      }
    }
    None
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    for provider in self {
      if let Some(v) = provider.hash_shader_group_key_with_self_type_info(any_id, hasher) {
        return Some(v);
      }
    }
    None
  }
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    any_idx: EntityHandle<StandardModelEntity>,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    ctx.next_key_scope_root();
    for (i, provider) in self.iter().enumerate() {
      if let Some(v) = ctx.keyed_scope(&i, |ctx| {
        provider.generate_indirect_draw_provider(batch, any_idx, ctx)
      }) {
        return Some(v);
      }
    }
    None
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    for provider in self {
      if let Some(builder) = provider.make_draw_command_builder(any_idx) {
        return Some(builder);
      }
    }
    None
  }
}
