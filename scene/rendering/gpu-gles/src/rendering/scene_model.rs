use crate::*;

pub struct GLESPreferredComOrderRendererProvider {
  pub scene_model_ids: QueryToken,
  pub node: BoxedQueryBasedGPUFeature<Box<dyn GLESNodeRenderImpl>>,
  pub model_impl: Vec<BoxedQueryBasedGPUFeature<Box<dyn GLESModelRenderImpl>>>,
}

type SceneModelIdUniforms = UniformUpdateContainer<EntityHandle<SceneModelEntity>, Vec4<u32>>;

impl QueryBasedFeature<Box<dyn SceneModelRenderer>> for GLESPreferredComOrderRendererProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let ids = global_watch()
      .watch_entity_set::<SceneModelEntity>()
      .key_as_value()
      .collective_map(|v| Vec4::new(v.into_raw().index(), 0, 0, 0))
      .into_query_update_uniform(0, cx);

    let ids = SceneModelIdUniforms::default().with_source(ids);

    self.scene_model_ids = qcx.register_multi_updater(ids);

    self.node.register(qcx, cx);
    self.model_impl.iter_mut().for_each(|i| i.register(qcx, cx));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.scene_model_ids);
    self.node.deregister(qcx);
    self.model_impl.iter_mut().for_each(|i| i.deregister(qcx));
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn SceneModelRenderer> {
    Box::new(GLESPreferredComOrderRenderer {
      scene_model_ids: cx.take_multi_updater_updated(self.scene_model_ids).unwrap(),
      model_impl: self.model_impl.iter().map(|i| i.create_impl(cx)).collect(),
      node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      node_render: self.node.create_impl(cx),
    })
  }
}

pub struct GLESPreferredComOrderRenderer {
  scene_model_ids: LockReadGuardHolder<SceneModelIdUniforms>,
  model_impl: Vec<Box<dyn GLESModelRenderImpl>>,
  node_render: Box<dyn GLESNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
}

struct SceneModelIdWriter<'a> {
  id: &'a UniformBufferDataView<Vec4<u32>>,
}
impl ShaderHashProvider for SceneModelIdWriter<'_> {
  shader_hash_type_id! {SceneModelIdWriter<'static>}
}

impl GraphicsShaderProvider for SceneModelIdWriter<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let id = binding.bind_by(&self.id);
      builder.register::<LogicalRenderEntityId>(id.load().x());
    })
  }
}

impl ShaderPassBuilder for SceneModelIdWriter<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.id);
  }
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

    let id = self.scene_model_ids.get(&idx).unwrap();
    let id = SceneModelIdWriter { id };
    let id = Box::new(id) as Box<dyn RenderComponent>;

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

    let contents: [BindingController<Box<dyn RenderComponent>>; 7] = [
      pass.into_assign_binding_index(0),
      tex.into_assign_binding_index(0),
      id.into_assign_binding_index(2),
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
