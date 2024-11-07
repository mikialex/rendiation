use crate::*;

pub trait IndirectModelShapeRenderImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder + '_>>;
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

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder + '_>> {
    for provider in self {
      if let Some(builder) = provider.make_draw_command_builder(any_idx) {
        return Some(builder);
      }
    }
    None
  }
}
