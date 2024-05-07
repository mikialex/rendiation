use crate::*;

pub trait GLESModelRenderImpl {
  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)>;
  fn material_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;
}

impl GLESModelRenderImpl for Vec<Box<dyn GLESModelRenderImpl>> {
  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
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
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(v) = provider.material_renderable(idx) {
        return Some(v);
      }
    }
    None
  }
}
pub struct DefaultSceneStdModelRendererProvider {
  pub materials: Vec<Box<dyn RenderImplProvider<Box<dyn GLESModelMaterialRenderImpl>>>>,
  pub shapes: Vec<Box<dyn RenderImplProvider<Box<dyn GLESModelShapeRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn GLESModelRenderImpl>> for DefaultSceneStdModelRendererProvider {
  fn register_resource(&mut self, source: &mut ConcurrentStreamContainer, cx: &GPUResourceCtx) {
    self
      .materials
      .iter_mut()
      .for_each(|p| p.register_resource(source, cx));
    self
      .shapes
      .iter_mut()
      .for_each(|p| p.register_resource(source, cx));
  }

  fn create_impl(&self, res: &ConcurrentStreamUpdateResult) -> Box<dyn GLESModelRenderImpl> {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read(),
      materials: self.materials.iter().map(|v| v.create_impl(res)).collect(),
      shapes: self.shapes.iter().map(|v| v.create_impl(res)).collect(),
    })
  }
}
struct SceneStdModelRenderer {
  model: ComponentReadView<SceneModelStdModelRenderPayload>,
  materials: Vec<Box<dyn GLESModelMaterialRenderImpl>>,
  shapes: Vec<Box<dyn GLESModelShapeRenderImpl>>,
}

impl GLESModelRenderImpl for SceneStdModelRenderer {
  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let model = self.model.get(idx)?;
    let idx = (*model)?;
    self.shapes.make_component(idx.into())
  }

  fn material_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let model = self.model.get(idx)?;
    let idx = (*model)?;
    self.materials.make_component(idx.into())
  }
}
