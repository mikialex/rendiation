use crate::*;

pub trait IndirectNodeInfoSceneModelAccess: ShaderHashProvider + dyn_clone::DynClone {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectNodeInfoSceneModelAccessInvocation>;
  fn bind(&self, builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(IndirectNodeInfoSceneModelAccess);

pub trait IndirectNodeInfoSceneModelAccessInvocation {
  fn get_node_info(
    &self,
    scene_model_id: Node<u32>,
    v: &mut dyn Fn(ShaderReadonlyPtrOf<NodeStorage>),
  );

  fn get_node_info_value(&self, scene_model_id: Node<u32>) -> Node<NodeStorage> {
    let node = zeroed_val::<NodeStorage>().make_local_var();
    self.get_node_info(scene_model_id, &mut |v| node.store(v.load()));
    node.load()
  }
}

impl<'a> ShaderHashProvider for &'a dyn IndirectNodeInfoSceneModelAccess {
  shader_hash_type_id!(&'static dyn IndirectNodeInfoSceneModelAccess);
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher);
  }
}

impl<'a> IndirectNodeInfoSceneModelAccess for &'a dyn IndirectNodeInfoSceneModelAccess {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectNodeInfoSceneModelAccessInvocation> {
    (**self).build(cx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    (**self).bind(builder);
  }
}

impl<'a> ShaderHashProvider for Box<dyn IndirectNodeInfoSceneModelAccess + 'a> {
  shader_hash_type_id!(&'static dyn IndirectNodeInfoSceneModelAccess);
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher);
  }
}

impl<'a> IndirectNodeInfoSceneModelAccess for Box<dyn IndirectNodeInfoSceneModelAccess + 'a> {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectNodeInfoSceneModelAccessInvocation> {
    (**self).build(cx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    (**self).bind(builder);
  }
}

/// Bridge [IndirectNodeInfoSceneModelAccess] to [RenderComponent]
pub struct NodeRenderComponent<T>(pub T);

impl<T: ShaderHashProvider> ShaderHashProvider for NodeRenderComponent<T> {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.0.hash_type_info(hasher);
    hasher.hash_type::<NodeRenderComponent<()>>();
  }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline(hasher);
  }
}

impl<T: IndirectNodeInfoSceneModelAccess> ShaderPassBuilder for NodeRenderComponent<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.bind(&mut ctx.binding);
  }
}

impl<T: IndirectNodeInfoSceneModelAccess> GraphicsShaderProvider for NodeRenderComponent<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let accessor = self.0.build(binding);
      let scene_model_id = builder.query::<RootLogicalRenderEntityId>();
      let node = accessor.get_node_info_value(scene_model_id).expand();

      register_or_compose_world_related_info(builder, node);

      // the RenderVertexPosition requires camera, so here we only process normal part
      if let Some(normal) = builder.try_query::<GeometryNormal>() {
        builder.register::<VertexRenderNormal>(node.normal_matrix * normal);
      }
    })
  }
}

pub trait IndirectNodeRenderImpl: dyn_clone::DynClone {
  fn make_component_indirect(&self) -> Option<Box<dyn IndirectNodeInfoSceneModelAccess>>;
  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(&self, hasher: &mut PipelineHasher) -> Option<()> {
    self.hash_shader_group_key(hasher).map(|_| {
      hasher.hash(self.as_any().type_id());
    })
  }

  fn as_any(&self) -> &dyn Any;
}
dyn_clone::clone_trait_object!(IndirectNodeRenderImpl);

pub fn use_node_storage(cx: &mut QueryGPUHookCx) -> Option<IndirectNodeRenderer> {
  let (cx, nodes) = cx.use_storage_buffer("nodes data", 128, u32::MAX);

  use_global_node_world_mat(cx)
    .into_delta_change()
    .map_changes(NodeStorage::from_world_mat)
    .update_storage_array(cx, nodes, 0);

  nodes.use_update(cx);
  nodes.use_max_item_count_by_db_entity::<SceneNodeEntity>(cx);

  let sm_to_node_device = use_db_device_foreign_key::<SceneModelRefNode>(cx);

  cx.when_render(|| IndirectNodeRenderer {
    sm_to_node: sm_to_node_device.unwrap(),
    node_to_node_data: nodes.get_gpu_buffer(),
  })
}

#[derive(Clone)]
pub struct IndirectNodeRenderer {
  pub sm_to_node: AbstractReadonlyStorageBuffer<[u32]>,
  pub node_to_node_data: AbstractReadonlyStorageBuffer<[NodeStorage]>,
}

impl IndirectNodeRenderImpl for IndirectNodeRenderer {
  fn make_component_indirect(&self) -> Option<Box<dyn IndirectNodeInfoSceneModelAccess>> {
    let node = NodeGPUStorage {
      sm_to_node: self.sm_to_node.clone(),
      node_to_node_data: self.node_to_node_data.clone(),
    };
    Some(Box::new(node))
  }
  fn hash_shader_group_key(&self, _: &mut PipelineHasher) -> Option<()> {
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

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

#[derive(Clone)]
pub struct NodeGPUStorage {
  sm_to_node: AbstractReadonlyStorageBuffer<[u32]>,
  node_to_node_data: AbstractReadonlyStorageBuffer<[NodeStorage]>,
}

impl ShaderHashProvider for NodeGPUStorage {
  shader_hash_type_id! {}
}

impl IndirectNodeInfoSceneModelAccess for NodeGPUStorage {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectNodeInfoSceneModelAccessInvocation> {
    struct Impl {
      sm_to_node: ShaderReadonlyPtrOf<[u32]>,
      node_to_node_data: ShaderReadonlyPtrOf<[NodeStorage]>,
    }
    impl IndirectNodeInfoSceneModelAccessInvocation for Impl {
      fn get_node_info(
        &self,
        scene_model_id: Node<u32>,
        v: &mut dyn Fn(ShaderReadonlyPtrOf<NodeStorage>),
      ) {
        let node_id = self.sm_to_node.index(scene_model_id).load();
        v(self.node_to_node_data.index(node_id))
      }
    }

    Box::new(Impl {
      sm_to_node: cx.bind_by(&self.sm_to_node),
      node_to_node_data: cx.bind_by(&self.node_to_node_data),
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.sm_to_node);
    builder.bind(&self.node_to_node_data);
  }
}

/// this logic is to support transform instanced model
pub fn register_or_compose_world_related_info(
  builder: &mut ShaderVertexBuilder,
  node: ENode<NodeStorage>,
) {
  // note, the branching is not need shader hash, it should be hashed by upstream injector.
  if let Some(pre_injected) = builder.try_query::<WorldNoneTranslationMatrix>() {
    builder
      .register::<WorldNoneTranslationMatrix>(pre_injected * node.world_matrix_none_translation);
  } else {
    builder.register::<WorldNoneTranslationMatrix>(node.world_matrix_none_translation);
  }

  let hpt = hpt_storage_to_hpt(node.world_position_hp);
  if let Some(pre_injected) = builder.try_query::<WorldPositionHP>() {
    builder.register::<WorldPositionHP>(hpt_compose_hpt(pre_injected, hpt));
  } else {
    builder.register::<WorldPositionHP>(hpt);
  }

  if let Some(pre_injected) = builder.try_query::<WorldNormalMatrix>() {
    builder.register::<WorldNormalMatrix>(pre_injected * node.normal_matrix);
  } else {
    builder.register::<WorldNormalMatrix>(node.normal_matrix);
  }
}
