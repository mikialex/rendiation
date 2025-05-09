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

#[derive(Default)]
pub struct DefaultSceneStdModelRendererProvider {
  pub materials: Vec<BoxedQueryBasedGPUFeature<Box<dyn GLESModelMaterialRenderImpl>>>,
  pub shapes: Vec<BoxedQueryBasedGPUFeature<Box<dyn GLESModelShapeRenderImpl>>>,
}

impl DefaultSceneStdModelRendererProvider {
  pub fn register_material_impl(
    mut self,
    imp: impl QueryBasedFeature<Box<dyn GLESModelMaterialRenderImpl>, Context = GPU> + 'static,
  ) -> Self {
    self.materials.push(Box::new(imp));
    self
  }
  pub fn register_shape_impl(
    mut self,
    imp: impl QueryBasedFeature<Box<dyn GLESModelShapeRenderImpl>, Context = GPU> + 'static,
  ) -> Self {
    self.shapes.push(Box::new(imp));
    self
  }
}

impl QueryBasedFeature<Box<dyn GLESModelRenderImpl>> for DefaultSceneStdModelRendererProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.materials.iter_mut().for_each(|p| p.register(qcx, cx));
    self.shapes.iter_mut().for_each(|p| p.register(qcx, cx));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    self.materials.iter_mut().for_each(|p| p.deregister(qcx));
    self.shapes.iter_mut().for_each(|p| p.deregister(qcx));
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn GLESModelRenderImpl> {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: self.materials.iter().map(|v| v.create_impl(cx)).collect(),
      shapes: self.shapes.iter().map(|v| v.create_impl(cx)).collect(),
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
