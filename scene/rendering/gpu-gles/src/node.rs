use crate::*;

pub fn use_node_uniforms(cx: &mut QueryGPUHookCx) -> Option<GLESNodeRenderer> {
  let uniform = cx.use_uniform_buffers2();

  use_global_node_world_mat(cx)
    .into_delta_change()
    .map_changes(NodeUniform::from_world_mat)
    .update_uniforms(&uniform, 0, cx.gpu);

  cx.when_render(|| GLESNodeRenderer(uniform.make_read_holder()))
}

pub struct GLESNodeRenderer(LockReadGuardHolder<SceneNodeUniforms>);

impl GLESNodeRenderer {
  pub fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPUUniform {
      ubo: self.0.get(&idx.into_raw())?,
    };
    Some(Box::new(node))
  }
}

type SceneNodeUniforms = UniformBufferCollectionRaw<RawEntityHandle, NodeUniform>;

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
        r.register_typed_both_stage::<WorldNoneTranslationMatrix>(
          node.world_matrix_none_translation,
        );
        r.register_typed_both_stage::<WorldPositionHP>(hpt_uniform_to_hpt(node.world_position_hp));
        r.register_typed_both_stage::<WorldNormalMatrix>(node.normal_matrix);
      })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct NodeUniform {
  pub world_matrix_none_translation: Mat4<f32>,
  pub world_position_hp: HighPrecisionTranslationUniform,
  pub normal_matrix: Shader16PaddedMat3,
}

impl NodeUniform {
  pub fn from_world_mat(world_matrix: Mat4<f64>) -> Self {
    let (world_matrix_none_translation, world_position_hp) =
      into_mat_hpt_uniform_pair(world_matrix);
    Self {
      world_matrix_none_translation,
      world_position_hp,
      normal_matrix: world_matrix.into_f32().to_normal_matrix().into(),
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

      builder.register::<WorldNoneTranslationMatrix>(node.world_matrix_none_translation);
      builder.register::<WorldPositionHP>(hpt_uniform_to_hpt(node.world_position_hp));
      builder.register::<WorldNormalMatrix>(node.normal_matrix);

      // the RenderVertexPosition requires camera, so here we only process normal part
      if let Some(normal) = builder.try_query::<GeometryNormal>() {
        builder.register::<VertexRenderNormal>(node.normal_matrix * normal);
      }
    })
  }
}

impl ShaderPassBuilder for NodeGPUUniform<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.ubo);
  }
}
