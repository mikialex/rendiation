use crate::*;

pub type SceneNodeUniforms = UniformUpdateContainer<EntityHandle<SceneNodeEntity>, NodeUniform>;

pub fn node_uniforms(cx: &GPU) -> SceneNodeUniforms {
  let source = scene_node_derive_world_mat()
    .collective_map(|mat| NodeUniform {
      world_matrix: mat,
      normal_matrix: mat.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    })
    .into_query_update_uniform(0, cx);

  SceneNodeUniforms::default().with_source(source)
}

pub struct NodeGPUUniform<'a> {
  pub ubo: &'a UniformBufferDataView<NodeUniform>,
}

impl NodeGPUUniform<'_> {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> GraphicsPairInputNodeAccessor<ShaderUniformPtr<NodeUniform>> {
    builder
      .bind_by_and_prepare(&self.ubo)
      .using_graphics_pair(|r, node| {
        let node = node.load().expand();
        r.register_typed_both_stage::<WorldMatrix>(node.world_matrix);
        r.register_typed_both_stage::<WorldNormalMatrix>(node.normal_matrix);
      })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct NodeUniform {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader16PaddedMat3,
}

impl NodeUniform {
  pub fn from_world_mat(world_matrix: Mat4<f32>) -> Self {
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl ShaderHashProvider for NodeGPUUniform<'_> {
  shader_hash_type_id! {NodeGPUUniform<'static>}
}

impl GraphicsShaderProvider for NodeGPUUniform<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let node = binding.bind_by(&self.ubo).load().expand();
      let position = builder.query::<GeometryPosition>();
      let position = node.world_matrix * (position, val(1.)).into();

      builder.register::<WorldMatrix>(node.world_matrix);
      builder.register::<WorldNormalMatrix>(node.normal_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      if let Some(normal) = builder.try_query::<GeometryNormal>() {
        builder.register::<WorldVertexNormal>(node.normal_matrix * normal);
      }
    })
  }
}

impl ShaderPassBuilder for NodeGPUUniform<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.ubo);
  }
}
