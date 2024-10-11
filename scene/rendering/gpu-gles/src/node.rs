use crate::*;

pub type SceneNodeUniforms = UniformUpdateContainer<EntityHandle<SceneNodeEntity>, NodeUniform>;

pub fn node_uniforms(cx: &GPU) -> SceneNodeUniforms {
  let source = scene_node_derive_world_mat()
    .collective_map(|mat| NodeUniform {
      world_matrix: mat,
      normal_matrix: mat.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    })
    .into_uniform_collection_update(0, cx);

  SceneNodeUniforms::default().with_source(source)
}

pub struct NodeGPUUniform<'a> {
  pub ubo: &'a UniformBufferDataView<NodeUniform>,
}

impl<'a> NodeGPUUniform<'a> {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<ShaderUniformPtr<NodeUniform>> {
    builder
      .bind_by_and_prepare(&self.ubo)
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

impl<'a> ShaderHashProvider for NodeGPUUniform<'a> {
  shader_hash_type_id! {NodeGPUUniform<'static>}
}

impl<'a> GraphicsShaderDependencyProvider for NodeGPUUniform<'a> {
  fn inject_shader_dependencies(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.inject_uniforms(builder);
  }
}

impl<'a> GraphicsShaderProvider for NodeGPUUniform<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let node = binding.bind_by(&self.ubo).load().expand();
      let position = builder.query::<GeometryPosition>()?;
      let position = node.world_matrix * (position, val(1.)).into();

      builder.register::<WorldMatrix>(node.world_matrix);
      builder.register::<WorldNormalMatrix>(node.normal_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      let normal = builder.query::<GeometryNormal>()?;
      builder.register::<WorldVertexNormal>(node.normal_matrix * normal);
      Ok(())
    })
  }
}

impl<'a> ShaderPassBuilder for NodeGPUUniform<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.ubo);
  }
}
