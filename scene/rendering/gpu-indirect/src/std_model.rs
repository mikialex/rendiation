use crate::*;

both!(IndirectSceneStdModelId, u32);
pub type SceneStdModelStorageBuffer = ReactiveStorageBufferContainer<SceneStdModelStorage>;

pub fn std_model_data(cx: &GPU) -> SceneStdModelStorageBuffer {
  let mesh = global_watch()
    .watch::<StandardModelRefAttributesMeshEntity>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX));
  let mesh_offset = offset_of!(SceneStdModelStorage, mesh);

  let material_flat = global_watch()
    .watch::<StandardModelRefFlatMaterial>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();

  let material_pbr_mr = global_watch()
    .watch::<StandardModelRefPbrMRMaterial>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();

  let material_pbr_sg = global_watch()
    .watch::<StandardModelRefPbrSGMaterial>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();

  let material = material_flat
    .collective_select(material_pbr_mr)
    .collective_select(material_pbr_sg);

  let material_offset = offset_of!(SceneStdModelStorage, material);

  ReactiveStorageBufferContainer::new(cx)
    .with_source(mesh, mesh_offset)
    .with_source(material, material_offset)
}

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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<IndirectSceneStdModelId>();
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectAbstractMeshId>(model.mesh);
      builder.register::<IndirectAbstractMaterialId>(model.material);
    })
  }
}

impl<'a> ShaderPassBuilder for StdModelGPUStorage<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
