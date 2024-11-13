use crate::*;

pub trait IndirectModelRenderImpl {
  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>>;

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
}

impl IndirectModelRenderImpl for Vec<Box<dyn IndirectModelRenderImpl>> {
  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(v) = provider.shape_renderable_indirect(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    for provider in self {
      if let Some(v) = provider.make_draw_command_builder(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(v) = provider.material_renderable_indirect(any_idx, cx) {
        return Some(v);
      }
    }
    None
  }
}

pub struct DefaultSceneStdModelRendererProvider {
  pub materials: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectModelMaterialRenderImpl>>>>,
  pub shapes: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectModelShapeRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn IndirectModelRenderImpl>> for DefaultSceneStdModelRendererProvider {
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

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn IndirectModelRenderImpl> {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: self.materials.iter().map(|v| v.create_impl(res)).collect(),
      shapes: self.shapes.iter().map(|v| v.create_impl(res)).collect(),
    })
  }
}
struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  materials: Vec<Box<dyn IndirectModelMaterialRenderImpl>>,
  shapes: Vec<Box<dyn IndirectModelShapeRenderImpl>>,
}

impl IndirectModelRenderImpl for SceneStdModelRenderer {
  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_component_indirect(model)
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_draw_command_builder(model)
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let model = self.model.get(any_idx)?;
    self.materials.make_component_indirect(model, cx)
  }
}
