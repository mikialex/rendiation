use crate::*;

both!(IndirectSceneStdModelId, u32);
pub type SceneStdModelStorageBuffer = ReactiveStorageBufferContainer<SceneStdModelStorage>;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct SceneStdModelStorage {
  pub mesh: u32,
  pub material: u32,
}

pub struct StdModelGPUStorage<'a> {
  pub buffer: &'a SceneStdModelStorageBuffer,
}

impl<'a> ShaderHashProvider for StdModelGPUStorage<'a> {
  shader_hash_type_id! {StdModelGPUStorage<'static>}
}

impl<'a> GraphicsShaderProvider for StdModelGPUStorage<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<IndirectSceneStdModelId>()?;
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectSceneAbstractMeshId>(model.mesh);
      builder.register::<IndirectSceneAbstractMaterialId>(model.material);
      Ok(())
    })
  }
}

impl<'a> ShaderPassBuilder for StdModelGPUStorage<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
