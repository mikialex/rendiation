use crate::*;

pub type SceneModelStorageBuffer = ReactiveStorageBufferContainer<SceneModelStorage>;

pub fn scene_model_data(cx: &GPU) -> SceneModelStorageBuffer {
  let std_model = global_watch()
    .watch::<SceneModelStdModelRenderPayload>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX));
  let std_model_offset = offset_of!(SceneModelStorage, std_model);

  let node = global_watch()
    .watch::<SceneModelRefNode>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();
  let node_offset = offset_of!(SceneModelStorage, node);

  ReactiveStorageBufferContainer::new(cx)
    .with_source(std_model, std_model_offset)
    .with_source(node, node_offset)
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

impl<'a> ShaderHashProvider for SceneModelGPUStorage<'a> {
  shader_hash_type_id! {SceneModelGPUStorage<'static>}
}

impl<'a> GraphicsShaderProvider for SceneModelGPUStorage<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<IndirectSceneModelId>();
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectSceneNodeId>(model.node);
      builder.register::<IndirectSceneStdModelId>(model.std_model);
    })
  }
}

impl<'a> ShaderPassBuilder for SceneModelGPUStorage<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
