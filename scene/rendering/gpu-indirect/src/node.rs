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

#[derive(Default)]
pub struct DefaultIndirectNodeRenderImplProvider {
  storage: QueryToken,
}
pub struct DefaultIndirectNodeRenderImpl {
  node_gpu: LockReadGuardHolder<MultiUpdateContainer<CommonStorageBufferImpl<NodeStorage>>>,
}

impl QueryBasedFeature<Box<dyn IndirectNodeRenderImpl>> for DefaultIndirectNodeRenderImplProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let storage = node_storages(cx);
    self.storage = qcx.register_multi_updater(storage);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.storage);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectNodeRenderImpl> {
    Box::new(DefaultIndirectNodeRenderImpl {
      node_gpu: cx.take_multi_updater_updated(self.storage).unwrap(),
    })
  }
}

impl IndirectNodeRenderImpl for DefaultIndirectNodeRenderImpl {
  fn make_component_indirect(
    &self,
    _any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPUStorage {
      buffer: &self.node_gpu,
    };
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

pub fn node_storages(cx: &GPU) -> ReactiveStorageBufferContainer<NodeStorage> {
  let source = scene_node_derive_world_mat()
    .collective_map(|mat| NodeStorage {
      world_matrix: mat,
      normal_matrix: mat.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    })
    .into_query_update_storage(0);

  create_reactive_storage_buffer_container(128, u32::MAX, cx).with_source(source)
}

pub struct NodeGPUStorage<'a> {
  pub buffer: &'a MultiUpdateContainer<CommonStorageBufferImpl<NodeStorage>>,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct NodeStorage {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader16PaddedMat3,
}

impl NodeStorage {
  pub fn from_world_mat(world_matrix: Mat4<f32>) -> Self {
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
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
      let nodes = binding.bind_by(self.buffer.inner.gpu());
      let current_node_id = builder.query::<IndirectSceneNodeId>();
      let node = nodes.index(current_node_id).load().expand();

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

impl ShaderPassBuilder for NodeGPUStorage<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
