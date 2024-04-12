use crate::*;

/// in gles  rendering, for each scene model, we need to create a render component and a draw
/// command;
pub trait GLESSceneModelRenderImpl {
  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand>;
  fn make_component(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponentAny,
  ) -> Option<Box<dyn RenderComponent>>;
}

pub struct GLESPreferredComOrderRendererProvider {
  pub node: Box<dyn RenderImplProvider<Box<dyn GLESNodeRenderImpl>>>,
}

impl GLESPreferredComOrderRendererProvider {
  //
}

impl RenderImplProvider<Box<dyn GLESSceneModelRenderImpl>>
  for GLESPreferredComOrderRendererProvider
{
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    todo!()
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESSceneModelRenderImpl> {
    todo!()
  }
}

pub struct GLESPreferredComOrderRenderer {
  model_impl: Vec<Box<dyn GLESModelRenderImpl>>,
  node_gpu: Box<dyn GLESNodeRenderImpl>,
  node: ComponentReadView<SceneModelRefNode>,
  camera_gpu: Box<dyn GLESCameraRenderImpl>,
}

impl GLESSceneModelRenderImpl for GLESPreferredComOrderRenderer {
  fn draw_command(&self, idx: AllocIdx<SceneModelEntity>) -> Option<DrawCommand> {
    self.model_impl.draw_command(idx)
  }

  fn make_component(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponentAny,
  ) -> Option<Box<dyn RenderComponent>> {
    let node = self.node.get(idx)?;
    let node = (*node)?;
    let node = self.node_gpu.make_component(node.into())?;

    let camera = self.camera_gpu.make_component(camera)?;

    let mesh = self.model_impl.material_renderable(idx)?;
    let material = self.model_impl.material_renderable(idx)?;

    let components: [&dyn RenderComponentAny; 5] = [
      &pass.assign_binding_index(0),
      &(&*mesh).assign_binding_index(2),
      &(&*node).assign_binding_index(2),
      &(&*camera).assign_binding_index(1),
      &(&*material).assign_binding_index(2),
    ];
    // Some(Box::new(components))
    todo!()
  }
}
