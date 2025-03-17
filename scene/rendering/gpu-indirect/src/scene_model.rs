use crate::*;

pub type SceneModelStorageBuffer = ReactiveStorageBufferContainer<SceneModelStorage>;

pub fn scene_model_data(cx: &GPU) -> SceneModelStorageBuffer {
  let std_model = global_watch()
    .watch::<SceneModelStdModelRenderPayload>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .into_query_update_storage(offset_of!(SceneModelStorage, std_model));

  let node = global_watch()
    .watch::<SceneModelRefNode>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_query_update_storage(offset_of!(SceneModelStorage, node));

  create_reactive_storage_buffer_container(128, u32::MAX, cx)
    .with_source(std_model)
    .with_source(node)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct SceneModelStorage {
  pub node: u32,
  pub std_model: u32,
}

pub struct SceneModelGPUStorage<'a> {
  pub buffer: &'a SceneModelStorageBuffer,
}

impl ShaderHashProvider for SceneModelGPUStorage<'_> {
  shader_hash_type_id! {SceneModelGPUStorage<'static>}
}

impl GraphicsShaderProvider for SceneModelGPUStorage<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<LogicalRenderEntityId>();
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectSceneNodeId>(model.node);
      builder.register::<IndirectSceneStdModelId>(model.std_model);
    })
  }
}

impl ShaderPassBuilder for SceneModelGPUStorage<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
