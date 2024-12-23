use crate::*;

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

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>>;
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
      if let Some(v) = provider.hash_shader_group_key(any_id, hasher) {
        return Some(v);
      }
    }
    None
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    for provider in self {
      if let Some(builder) = provider.make_draw_command_builder(any_idx) {
        return Some(builder);
      }
    }
    None
  }
}
