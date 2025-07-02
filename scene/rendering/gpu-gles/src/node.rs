use crate::*;

pub fn use_node_uniforms(cx: &mut impl QueryGPUHookCx) -> Option<GLESNodeRenderer> {
  cx.use_uniform_buffers(|source, cx| {
    source.with_source(
      scene_node_derive_world_mat()
        .collective_map(NodeUniform::from_world_mat)
        .into_query_update_uniform(0, cx),
    )
  })
  .map(GLESNodeRenderer)
}

pub struct GLESNodeRenderer(LockReadGuardHolder<SceneNodeUniforms>);

impl GLESNodeRenderer {
  pub fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPUUniform {
      ubo: self.0.get(&idx)?,
    };
    Some(Box::new(node))
  }
}

type SceneNodeUniforms = UniformUpdateContainer<EntityHandle<SceneNodeEntity>, NodeUniform>;

pub struct NodeGPUUniform<'a> {
  pub ubo: &'a UniformBufferDataView<NodeUniform>,
}

impl NodeGPUUniform<'_> {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> GraphicsPairInputNodeAccessor<UniformBufferDataView<NodeUniform>> {
    builder
      .bind_by_and_prepare(self.ubo)
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
  pub fn from_world_mat(world_matrix: Mat4<f64>) -> Self {
    let world_matrix = world_matrix.map(|v| v as f32);
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
