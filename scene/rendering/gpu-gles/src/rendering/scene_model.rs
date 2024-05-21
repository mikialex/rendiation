use crate::*;

pub struct GLESPreferredComOrderRendererProvider {
  pub node: Box<dyn RenderImplProvider<Box<dyn GLESNodeRenderImpl>>>,
  pub camera: Box<dyn RenderImplProvider<Box<dyn GLESCameraRenderImpl>>>,
  pub model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn GLESModelRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn SceneModelRenderer>> for GLESPreferredComOrderRendererProvider {
  fn register_resource(&mut self, source: &mut ReactiveStateJoinUpdater, cx: &GPUResourceCtx) {
    self.node.register_resource(source, cx);
    self.camera.register_resource(source, cx);
    self
      .model_impl
      .iter_mut()
      .for_each(|i| i.register_resource(source, cx));
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn SceneModelRenderer> {
    Box::new(GLESPreferredComOrderRenderer {
      model_impl: self.model_impl.iter().map(|i| i.create_impl(res)).collect(),
      node: global_entity_component_of::<SceneModelRefNode>().read(),
      node_render: self.node.create_impl(res),
      camera_gpu: self.camera.create_impl(res),
    })
  }
}

pub struct GLESPreferredComOrderRenderer {
  model_impl: Vec<Box<dyn GLESModelRenderImpl>>,
  node_render: Box<dyn GLESNodeRenderImpl>,
  node: ComponentReadView<SceneModelRefNode>,
  camera_gpu: Box<dyn GLESCameraRenderImpl>,
}

impl SceneModelRenderer for GLESPreferredComOrderRenderer {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    let node = self.node.get(idx)?;
    let node = (*node)?.index();
    let node = self.node_render.make_component(node.into())?;

    let camera = self.camera_gpu.make_component(camera)?;

    let (shape, draw) = self.model_impl.shape_renderable(idx)?;
    let material = self.model_impl.material_renderable(idx, tex)?;

    let pass = Box::new(pass) as Box<dyn RenderComponent + 'a>;

    let contents: [BindingController<Box<dyn RenderComponent + 'a>>; 5] = [
      pass.into_assign_binding_index(0),
      shape.into_assign_binding_index(2),
      node.into_assign_binding_index(2),
      camera.into_assign_binding_index(1),
      material.into_assign_binding_index(2),
    ];

    let render = Box::new(RenderArray { contents }) as Box<dyn RenderComponent>;
    Some((render, draw))
  }
}
