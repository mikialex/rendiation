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

pub fn std_model_renderer(
  cx: &mut QueryGPUHookCx,
  materials: Option<Box<dyn GLESModelMaterialRenderImpl>>,
  shapes: Option<Box<dyn GLESModelShapeRenderImpl>>,
) -> Option<Box<dyn GLESModelRenderImpl>> {
  cx.when_render(|| {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: materials.unwrap(),
      shapes: shapes.unwrap(),
    }) as Box<dyn GLESModelRenderImpl>
  })
}

struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  materials: Box<dyn GLESModelMaterialRenderImpl>,
  shapes: Box<dyn GLESModelShapeRenderImpl>,
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
