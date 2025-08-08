use crate::*;

pub fn use_gles_scene_model_renderer(
  cx: &mut QueryGPUHookCx,
  model_impl: Option<Box<dyn GLESModelRenderImpl>>,
) -> Option<Box<dyn SceneModelRenderer>> {
  let node_render = use_node_uniforms(cx);

  let scene_model_ids =
    cx.use_uniform_buffers::<EntityHandle<SceneModelEntity>, Vec4<u32>>(|source, cx| {
      source.with_source(
        global_watch()
          .watch_entity_set::<SceneModelEntity>()
          .key_as_value()
          .collective_map(|v| Vec4::new(v.into_raw().index(), 0, 0, 0))
          .into_query_update_uniform(0, cx),
      )
    });

  cx.when_render(|| {
    Box::new(GLESPreferredComOrderRenderer {
      scene_model_ids: scene_model_ids.unwrap(),
      model_impl: model_impl.unwrap(),
      node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      node_render: node_render.unwrap(),
    }) as Box<_>
  })
}

type SceneModelIdUniforms = UniformUpdateContainer<EntityHandle<SceneModelEntity>, Vec4<u32>>;

pub struct GLESPreferredComOrderRenderer {
  scene_model_ids: LockReadGuardHolder<SceneModelIdUniforms>,
  model_impl: Box<dyn GLESModelRenderImpl>,
  node_render: GLESNodeRenderer,
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
    let id = &id as &dyn RenderComponent;

    let node = self.node.get(idx).ok_or(E::NodeAccessFailed(idx))?;
    let node = self
      .node_render
      .make_component(node)
      .ok_or(E::NodeGPUAccessFailed(node))?;
    let node = node.as_ref();

    let camera = &camera as &dyn RenderComponent;

    let (shape, draw) = self
      .model_impl
      .shape_renderable(idx)
      .ok_or(E::ShapeGPUAccessFailed(idx))?;
    let shape = shape.as_ref();

    let material = self
      .model_impl
      .material_renderable(idx, tex)
      .ok_or(E::MaterialGPUAccessFailed(idx))?;
    let material = material.as_ref();

    let pass = pass as &dyn RenderComponent;
    let tex = &GPUTextureSystemAsRenderComponent(tex) as &dyn RenderComponent;

    let contents: [BindingController<&dyn RenderComponent>; 7] = [
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
