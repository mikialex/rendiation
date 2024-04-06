use crate::*;

pub type SceneNodeUniforms = UniformUpdateContainer<SceneNodeEntity, TransformGPUData>;

pub fn node_gpus(
  node_mats: impl ReactiveCollection<AllocIdx<SceneNodeEntity>, Mat4<f32>>,
  cx: &GPUResourceCtx,
) -> SceneNodeUniforms {
  let source = node_mats
    .collective_map(|mat| TransformGPUData {
      world_matrix: mat,
      normal_matrix: mat.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    })
    .into_uniform_collection_update(0, cx);

  SceneNodeUniforms::default().with_source(source)
}

pub struct NodeGPU<'a> {
  pub ubo: &'a UniformBufferDataView<TransformGPUData>,
}

impl<'a> NodeGPU<'a> {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<ShaderUniformPtr<TransformGPUData>> {
    builder
      .bind_by(&self.ubo)
      .using_graphics_pair(builder, |r, node| {
        let node = node.load().expand();
        r.register_typed_both_stage::<WorldMatrix>(node.world_matrix);
        r.register_typed_both_stage::<WorldNormalMatrix>(node.normal_matrix);
      })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct TransformGPUData {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader16PaddedMat3,
}

impl TransformGPUData {
  pub fn from_world_mat(world_matrix: Mat4<f32>) -> Self {
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl<'a> ShaderHashProvider for NodeGPU<'a> {}

impl<'a> GraphicsShaderProvider for NodeGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let model = binding.bind_by(&self.ubo).load().expand();
      let position = builder.query::<GeometryPosition>()?;
      let position = model.world_matrix * (position, val(1.)).into();

      builder.register::<WorldMatrix>(model.world_matrix);
      builder.register::<WorldNormalMatrix>(model.normal_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      let normal = builder.query::<GeometryNormal>()?;
      builder.register::<WorldVertexNormal>(model.normal_matrix * normal);
      Ok(())
    })
  }
}

impl<'a> ShaderPassBuilder for NodeGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.ubo);
  }
}
