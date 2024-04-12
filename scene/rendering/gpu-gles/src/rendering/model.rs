use crate::*;

pub trait GLESModelRenderImpl {
  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand>;
  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>>;
  fn material_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>>;
}

impl GLESModelRenderImpl for Vec<Box<dyn GLESModelRenderImpl>> {
  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand> {
    for provider in self {
      if let Some(command) = provider.draw_command(idx) {
        return Some(command);
      }
    }
    None
  }

  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>> {
    for provider in self {
      if let Some(v) = provider.shape_renderable(idx) {
        return Some(v);
      }
    }
    None
  }

  fn material_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>> {
    for provider in self {
      if let Some(v) = provider.shape_renderable(idx) {
        return Some(v);
      }
    }
    None
  }
}

struct SceneStdModelRenderer {
  model: ComponentReadView<SceneModelStdModelRenderPayload>,
}

impl GLESModelRenderImpl for SceneStdModelRenderer {
  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand> {
    todo!()
  }

  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>> {
    todo!()
  }

  fn material_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>> {
    todo!()
  }
}
