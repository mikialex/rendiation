use crate::*;

pub struct GLESPreferredComOrderRendererProvider {
  pub node: Box<dyn RenderImplProvider<Box<dyn GLESNodeRenderImpl>>>,
  pub model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn GLESModelRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn SceneModelRenderer>> for GLESPreferredComOrderRendererProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.node.register_resource(source, cx);
    self
      .model_impl
      .iter_mut()
      .for_each(|i| i.register_resource(source, cx));
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.node.deregister_resource(source);
    self
      .model_impl
      .iter_mut()
      .for_each(|i| i.deregister_resource(source));
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn SceneModelRenderer> {
    Box::new(GLESPreferredComOrderRenderer {
      model_impl: self.model_impl.iter().map(|i| i.create_impl(res)).collect(),
      node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      node_render: self.node.create_impl(res),
    })
  }
}

pub struct GLESPreferredComOrderRenderer {
  model_impl: Vec<Box<dyn GLESModelRenderImpl>>,
  node_render: Box<dyn GLESNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
}

#[derive(thiserror::Error, Debug)]
pub enum GLESPreferredComOrderRendererRenderError {
  #[error("failed to get node instance from sm idx: {0}, the node reference maybe corrupted")]
  NodeAccessFailed(EntityHandle<SceneModelEntity>),
  #[error("failed to get node renderer from node idx:{0}")]
  NodeGPUAccessFailed(EntityHandle<SceneNodeEntity>),
  #[error("failed to get shape renderer from sm idx{0}")]
  ShapeGPUAccessFailed(EntityHandle<SceneModelEntity>),
  #[error("failed to get material renderer from sm idx{0}")]
  MaterialGPUAccessFailed(EntityHandle<SceneModelEntity>),
}
impl From<GLESPreferredComOrderRendererRenderError> for UnableToRenderSceneModelError {
  fn from(value: GLESPreferredComOrderRendererRenderError) -> Self {
    Self::FoundImplButUnableToRender(Box::new(value))
  }
}

impl SceneModelRenderer for GLESPreferredComOrderRenderer {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    use GLESPreferredComOrderRendererRenderError as E;
    let node = self.node.get(idx).ok_or(E::NodeAccessFailed(idx))?;
    let node = self
      .node_render
      .make_component(node)
      .ok_or(E::NodeGPUAccessFailed(node))?;

    let camera = Box::new(camera) as Box<dyn RenderComponent>;

    let (shape, draw) = self
      .model_impl
      .shape_renderable(idx)
      .ok_or(E::ShapeGPUAccessFailed(idx))?;
    let material = self
      .model_impl
      .material_renderable(idx, tex)
      .ok_or(E::MaterialGPUAccessFailed(idx))?;

    let pass = Box::new(pass) as Box<dyn RenderComponent>;
    let tex = Box::new(GPUTextureSystemAsRenderComponent(tex)) as Box<dyn RenderComponent>;

    let contents: [BindingController<Box<dyn RenderComponent>>; 6] = [
      pass.into_assign_binding_index(0),
      tex.into_assign_binding_index(0),
      shape.into_assign_binding_index(2),
      node.into_assign_binding_index(2),
      camera.into_assign_binding_index(1),
      material.into_assign_binding_index(2),
    ];

    let render = Box::new(RenderArray(contents)) as Box<dyn RenderComponent>;

    render.render(cx, draw);
    Ok(())
  }
}
