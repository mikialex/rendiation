use crate::*;

pub type SceneModelStorageBuffer = ReactiveStorageBufferContainer<SceneModelStorage>;

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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<IndirectSceneModelId>()?;
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectSceneNodeId>(model.node);
      builder.register::<IndirectSceneStdModelId>(model.std_model);
      Ok(())
    })
  }
}

impl<'a> ShaderPassBuilder for SceneModelGPUStorage<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
