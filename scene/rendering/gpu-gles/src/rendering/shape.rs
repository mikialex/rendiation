use crate::*;

pub trait GLESModelShapeRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>>;

  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand>;
}

impl GLESModelShapeRenderImpl for Vec<Box<dyn GLESModelShapeRenderImpl>> {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>> {
    for provider in self {
      if let Some(com) = provider.make_component(idx) {
        return Some(com);
      }
    }
    None
  }

  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand> {
    for provider in self {
      if let Some(command) = provider.draw_command(idx) {
        return Some(command);
      }
    }
    None
  }
}

pub struct AttributeMeshDefaultRenderImplProvider;

impl RenderImplProvider<Box<dyn GLESModelShapeRenderImpl>>
  for AttributeMeshDefaultRenderImplProvider
{
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    todo!()
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESModelShapeRenderImpl> {
    todo!()
  }
}

pub struct AttributeMeshDefaultRenderImpl {
  //
}

impl GLESModelShapeRenderImpl for AttributeMeshDefaultRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>> {
    todo!()
  }

  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand> {
    todo!()
  }
}
