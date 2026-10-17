use crate::*;

pub struct GLESModelRenderComponents<'a> {
  pub shape: Box<dyn RenderComponent + 'a>,
  pub draw: DrawCommand,
  pub material: Box<dyn RenderComponent + 'a>,
}

pub trait GLESModelRenderImpl {
  /// Return None if this impl is not responsible for the model. The responsibility is decided by
  /// the kind of the model only, so the impls are mutually exclusive. Once selected, missing or
  /// invalid data is reported as error.
  fn make_components<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Result<GLESModelRenderComponents<'a>, SceneModelRenderDataError>>;
}

impl GLESModelRenderImpl for Vec<Box<dyn GLESModelRenderImpl>> {
  fn make_components<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Result<GLESModelRenderComponents<'a>, SceneModelRenderDataError>> {
    self
      .iter()
      .find_map(|provider| provider.make_components(idx, cx))
  }
}

pub fn std_model_renderer(
  cx: &mut QueryGPUHookCx,
  materials: Option<Box<dyn GLESModelMaterialRenderImpl>>,
  shapes: Option<Box<dyn GLESModelShapeRenderImpl>>,
  revere_z: bool,
) -> Option<SceneStdModelRenderer> {
  let skin_gpu = use_skin(cx);

  let state_override = use_state_overrides(cx, revere_z);

  cx.when_render(|| SceneStdModelRenderer {
    model: read_global_db_foreign_key(),
    materials: materials.unwrap(),
    shapes: shapes.unwrap(),
    skin_gpu: skin_gpu.unwrap(),
    states: state_override.unwrap(),
    skin: read_global_db_foreign_key(),
  })
}

pub struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  materials: Box<dyn GLESModelMaterialRenderImpl>,
  shapes: Box<dyn GLESModelShapeRenderImpl>,
  skin_gpu: LockReadGuardHolder<SkinBoneMatrixesGPU>,
  skin: ForeignKeyReadView<StandardModelRefSkin>,
  states: StateOverrides,
}

impl SceneStdModelRenderer {
  fn create_components<'a>(
    &'a self,
    model: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Result<GLESModelRenderComponents<'a>, SceneModelRenderDataError> {
    let err = SceneModelRenderDataError;

    let (base_shape, draw) = self
      .shapes
      .make_component(model)
      .ok_or(err("no shape impl"))??;
    let state = self.states.get_gpu(model).ok_or(err("state override"))?;

    let shape = if let Some(skin) = self.skin.get(model) {
      let bones = self
        .skin_gpu
        .get_bone_provider(skin)
        .ok_or(err("skin bone matrices"))?;
      let bones = Box::new(bones) as Box<dyn RenderComponent>;
      let render = RenderArray([
        bones,
        base_shape,
        Box::new(state),
        Box::new(SkinVertexTransform),
      ]);

      Box::new(render)
    } else {
      base_shape
    };

    let material = self
      .materials
      .make_component(model, cx)
      .ok_or(err("no material impl"))??;

    Ok(GLESModelRenderComponents {
      shape,
      draw,
      material,
    })
  }
}

impl GLESModelRenderImpl for SceneStdModelRenderer {
  fn make_components<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Result<GLESModelRenderComponents<'a>, SceneModelRenderDataError>> {
    let model = self.model.get(idx)?;
    Some(self.create_components(model, cx))
  }
}
