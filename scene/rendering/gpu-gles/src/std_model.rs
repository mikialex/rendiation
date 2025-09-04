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
  let skin_gpu = use_skin(cx);

  cx.when_render(|| {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: materials.unwrap(),
      shapes: shapes.unwrap(),
      skin_gpu: skin_gpu.unwrap(),
      skin: global_entity_component_of::<StandardModelRefSkin>().read_foreign_key(),
    }) as Box<dyn GLESModelRenderImpl>
  })
}

struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  materials: Box<dyn GLESModelMaterialRenderImpl>,
  shapes: Box<dyn GLESModelShapeRenderImpl>,
  skin_gpu: LockReadGuardHolder<SkinBoneMatrixesGPU>,
  skin: ForeignKeyReadView<StandardModelRefSkin>,
}

impl GLESModelRenderImpl for SceneStdModelRenderer {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let model = self.model.get(idx)?;

    let (base_shape, cmd) = self.shapes.make_component(model)?;

    let shape = if let Some(skin) = self.skin.get(model) {
      let bones = self.skin_gpu.get_bone_provider(skin).unwrap();
      let bones = Box::new(bones) as Box<dyn RenderComponent>;
      // let applier = todo!(); // SkinVertexTransform
      let render = RenderArray([bones, base_shape]);

      Box::new(render)
    } else {
      base_shape
    };

    (shape, cmd).into()
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
