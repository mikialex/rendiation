use crate::*;

pub trait IndirectNodeRenderImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneNodeEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<SceneNodeEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      self.as_any().type_id().hash(hasher);
    })
  }

  fn as_any(&self) -> &dyn Any;
}

pub fn use_node_storage(cx: &mut QueryGPUHookCx) -> Option<IndirectNodeRenderer> {
  let (cx, nodes) = cx.use_storage_buffer(128, u32::MAX);

  use_global_node_world_mat(cx)
    .into_delta_change()
    .map_changes(NodeStorage::from_world_mat)
    .use_assure_result(cx)
    .update_storage_array(nodes, 0);

  cx.when_render(|| IndirectNodeRenderer(nodes.get_gpu_buffer()))
}

pub struct IndirectNodeRenderer(StorageBufferReadonlyDataView<[NodeStorage]>);

impl IndirectNodeRenderImpl for IndirectNodeRenderer {
  fn make_component_indirect(
    &self,
    _any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPUStorage(&self.0);
    Some(Box::new(node))
  }
  fn hash_shader_group_key(
    &self,
    _: EntityHandle<SceneNodeEntity>,
    _: &mut PipelineHasher,
  ) -> Option<()> {
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

only_vertex!(IndirectSceneNodeId, u32);

pub struct NodeGPUStorage<'a>(&'a StorageBufferReadonlyDataView<[NodeStorage]>);

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct NodeStorage {
  pub world_matrix_none_translation: Mat4<f32>,
  pub world_position_hp: HighPrecisionTranslationStorage,
  pub normal_matrix: Shader16PaddedMat3,
}

impl NodeStorage {
  pub fn from_world_mat(world_matrix: Mat4<f64>) -> Self {
    let (world_matrix_none_translation, world_position_hp) =
      into_mat_hpt_storage_pair(world_matrix);
    Self {
      world_matrix_none_translation,
      world_position_hp,
      normal_matrix: world_matrix.into_f32().to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl ShaderHashProvider for NodeGPUStorage<'_> {
  shader_hash_type_id! {NodeGPUStorage<'static>}
}

impl GraphicsShaderProvider for NodeGPUStorage<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let nodes = binding.bind_by(self.0);
      let current_node_id = builder.query::<IndirectSceneNodeId>();
      let node = nodes.index(current_node_id).load().expand();

      builder.register::<WorldNoneTranslationMatrix>(node.world_matrix_none_translation);
      builder.register::<WorldPositionHP>(hpt_storage_to_hpt(node.world_position_hp));
      builder.register::<WorldNormalMatrix>(node.normal_matrix);

      // the RenderVertexPosition requires camera, so here we only process normal part
      if let Some(normal) = builder.try_query::<GeometryNormal>() {
        builder.register::<VertexRenderNormal>(node.normal_matrix * normal);
      }
    })
  }
}

impl ShaderPassBuilder for NodeGPUStorage<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.0);
  }
}
