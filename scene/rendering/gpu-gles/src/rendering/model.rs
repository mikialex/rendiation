use crate::*;

pub trait GLESModelRenderImpl {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)>;
  fn material_renderable<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
}

impl GLESModelRenderImpl for Vec<Box<dyn GLESModelRenderImpl>> {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    for provider in self {
      if let Some(v) = provider.shape_renderable(idx) {
        return Some(v);
      }
    }
    None
  }

  fn material_renderable<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(v) = provider.material_renderable(idx, cx) {
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
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self
      .materials
      .iter_mut()
      .for_each(|p| p.register_resource(source, cx));
    self
      .shapes
      .iter_mut()
      .for_each(|p| p.register_resource(source, cx));
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self
      .materials
      .iter_mut()
      .for_each(|p| p.deregister_resource(source));
    self
      .shapes
      .iter_mut()
      .for_each(|p| p.deregister_resource(source));
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn GLESModelRenderImpl> {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: self.materials.iter().map(|v| v.create_impl(res)).collect(),
      shapes: self.shapes.iter().map(|v| v.create_impl(res)).collect(),
    })
  }
}
struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  materials: Vec<Box<dyn GLESModelMaterialRenderImpl>>,
  shapes: Vec<Box<dyn GLESModelShapeRenderImpl>>,
}

impl GLESModelRenderImpl for SceneStdModelRenderer {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let model = self.model.get(idx)?;
    self.shapes.make_component(model)
  }

  fn material_renderable<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let model = self.model.get(idx)?;
    self.materials.make_component(model, cx)
  }
}
