use std::hash::Hash;

use fast_hash_collection::fast_hash_scope;
use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;
use rendiation_scene_batch_extractor::SceneModelGroupKey;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod draw_cmd;
use draw_cmd::*;
use rendiation_webgpu_midc_downgrade::require_midc_downgrade;

use crate::*;

pub fn use_transform_instanced_model_group_key(
  cx: &mut QueryGPUHookCx,
  internal: UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>>,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>> {
  let sm_ref_instance = cx.use_db_rev_ref_tri_view::<SceneModelTransformInstancedModelPayload>();

  let instance_ref_source_sm = cx.use_db_rev_ref_tri_view::<TransformInstancedModelRefSceneModel>();

  let key = internal
    .fanout(instance_ref_source_sm, cx)
    .dual_query_map(|key| {
      let require_alpha_blend = key.require_alpha_blend();
      let internal = fast_hash_scope(|hasher| {
        std::any::TypeId::of::<TransformInstancedModelEntity>().hash(hasher);
        key.hash(hasher);
      });
      SceneModelGroupKey::ForeignHash {
        internal,
        require_alpha_blend,
      }
    });

  key.fanout(sm_ref_instance, cx).dual_query_boxed()
}

/// the input source_model_vertices_count's key is SceneModel type.
pub fn use_transform_instanced_model_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  force_midc_downgrade: bool,
  source_model_vertices_count: UseResult<
    impl DualQueryLike<Key = RawEntityHandle, Value = u32> + 'static,
  >,
) -> Option<TransformInstancedModelIndirectRendererBase> {
  let data_source = cx
    .use_dual_query::<TransformInstancedModelInstanceBuffer>()
    .map_spawn_stage_in_thread_dual_query(cx, move |source_info| {
      source_info.delta().into_change().collective_map(|buffer| {
        let new_buffer = buffer
          .iter()
          .map(|v| NodeStorage::from_world_mat(v.into_f64()))
          .collect();
        ExternalRefPtr::new(new_buffer)
      })
    });

  let (instance_buffer, allocation_info) = use_range_allocated_device_buffers::<NodeStorage>(
    cx,
    "instance transform pool",
    100,
    u32::MAX,
    data_source,
  );

  let range_change =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());

  let (cx, params) = cx.use_storage_buffer_with_host_backup::<InstanceMetaData>(
    "transform instance model metadata",
    128,
    u32::MAX,
  );

  // count is also covered
  let offset = std::mem::offset_of!(InstanceMetaData, instance_offset);
  range_change.update_storage_array_with_host(cx, params, offset);

  // todo, consider node world of instance model
  let change = cx
    .use_changes::<TransformInstancedModelPerUnitTransform>()
    .map_changes(|v| NodeStorage::from_world_mat(v.unwrap_or(Mat4::identity()).into_f64()));
  let offset = std::mem::offset_of!(InstanceMetaData, owned_transform);
  change.update_storage_array_with_host(cx, params, offset);

  let change = cx
    .use_changes::<TransformInstancedModelRefSceneModel>()
    .map_changes(map_raw_handle_or_u32_max);
  let offset = std::mem::offset_of!(InstanceMetaData, origin_model);
  change.update_storage_array_with_host(cx, params, offset);

  let offset = std::mem::offset_of!(InstanceMetaData, instance_vertices_count);
  let (source_model_vertices_count, sm_vc) = source_model_vertices_count.fork();

  let source_model_vertices_count = source_model_vertices_count
    .fanout(
      cx.use_db_rev_ref_tri_view::<TransformInstancedModelRefSceneModel>(),
      cx,
    )
    .into_delta_change();
  source_model_vertices_count.update_storage_array_with_host(cx, params, offset);

  params.use_max_item_count_by_db_entity::<TransformInstancedModelEntity>(cx);
  params.use_update(cx);

  let model_to_instance = use_db_device_foreign_key::<SceneModelTransformInstancedModelPayload>(cx);

  let sm_vc = sm_vc.use_assure_result(cx);

  cx.when_render(|| TransformInstancedModelIndirectRendererBase {
    source_model_vertices_count: sm_vc.expect_resolve_stage().view().into_boxed(),
    instance_buffer,
    instance_meta: params.get_gpu_buffer(),
    instance_meta_host: params.buffer.make_read_holder(),
    model_to_instance: model_to_instance.unwrap(),
    instance_model: read_global_db_foreign_key(),
    source_model: read_global_db_foreign_key(),
    used_in_midc_downgrade: require_midc_downgrade(cx.gpu.info(), force_midc_downgrade),
  })
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
struct InstanceMetaData {
  pub owned_transform: NodeStorage,
  pub instance_offset: u32,
  pub instance_count: u32,
  pub origin_model: u32,
  pub instance_vertices_count: u32,
}

pub struct TransformInstancedModelIndirectRendererBase {
  source_model_vertices_count: BoxedDynQuery<RawEntityHandle, u32>,
  instance_buffer: AbstractReadonlyStorageBuffer<[NodeStorage]>,
  instance_meta: AbstractReadonlyStorageBuffer<[InstanceMetaData]>,
  instance_meta_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<InstanceMetaData>>,
  model_to_instance: AbstractReadonlyStorageBuffer<[u32]>,
  instance_model: ForeignKeyReadView<SceneModelTransformInstancedModelPayload>,
  source_model: ForeignKeyReadView<TransformInstancedModelRefSceneModel>,
  used_in_midc_downgrade: bool,
}

impl TransformInstancedModelIndirectRendererBase {
  pub fn assert_base_model_count(&self, source_model_sm_id: EntityHandle<SceneModelEntity>) {
    if self
      .source_model_vertices_count
      .access(source_model_sm_id.raw_handle_ref())
      .is_none()
    {
      panic!("source model vertices count not found, this scene model not support instance draw");
    }
  }
}

pub struct TransformInstancedModelIndirectRenderer<T> {
  pub internal: T,
  pub base: TransformInstancedModelIndirectRendererBase,
}

impl<T> IndirectModelRenderImpl for TransformInstancedModelIndirectRenderer<T>
where
  T: IndirectModelRenderImpl + 'static,
{
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let instance_model = self.base.instance_model.get(any_id)?;
    let source_model = self.base.source_model.get(instance_model)?;
    self.internal.hash_shader_group_key(source_model, hasher)?;
    self.base.assert_base_model_count(source_model);
    hasher.hash(true); // distinguish between instanced and non-instanced
    Some(())
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>> {
    let instance_model = self.base.instance_model.get(any_idx)?;
    let _ = self.base.source_model.get(instance_model)?;
    // as we force use draw array, return none in any case
    Some(None)
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn model_info_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let instance_model = self.base.instance_model.get(any_idx)?;
    let source_model = self.base.source_model.get(instance_model)?;
    let internal = self.internal.model_info_injector(source_model)?;

    Some(Box::new(Override {
      instance_buffer: self.base.instance_buffer.clone(),
      instance_meta: self.base.instance_meta.clone(),
      model_to_instance: self.base.model_to_instance.clone(),
      internal,
    }))
  }

  fn shape_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let instance_model = self.base.instance_model.get(any_idx)?;
    let source_model = self.base.source_model.get(instance_model)?;
    self.internal.shape_renderable_indirect(source_model, cx)
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    let draw_command_builder = self.make_draw_command_builder(batch.impl_select_id)?;

    ctx
      .access_parallel_compute(|cx| {
        batch.create_default_indirect_draw_provider(
          draw_command_builder,
          cx,
          self.base.used_in_midc_downgrade,
        )
      })
      .into()
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    let instance_model = self.base.instance_model.get(any_idx)?;
    let source_model = self.base.source_model.get(instance_model)?;

    let internal = self.internal.make_draw_command_builder(source_model)?;

    let internal = match internal {
      DrawCommandBuilder::Indexed { .. } => assert_none_indexed(),
      DrawCommandBuilder::NoneIndexed(r) => r,
    };

    Some(DrawCommandBuilder::NoneIndexed(Box::new(
      InstanceDrawCommandBuilder {
        internal,
        instance_meta: self.base.instance_meta.clone(),
        model_to_instance_host: self.base.instance_model.clone(),
        source_model_host: self.base.source_model.clone(),
        model_to_instance: self.base.model_to_instance.clone(),
        instance_meta_host: self.base.instance_meta_host.clone(),
      },
    )))
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let instance_model = self.base.instance_model.get(any_idx)?;
    let source_model = self.base.source_model.get(instance_model)?;

    self.internal.material_renderable_indirect(source_model, cx)
  }
}

fn assert_none_indexed() -> ! {
  // we can not support index, because when do indexed draw, we can not get
  // incremental(from zero) vertex_index in vertex shader. and index draw rely on internal base index info
  //
  // perhaps there is another way to hack, but not worth to do it.
  unreachable!("source model must not be indexed in transform instanced model")
}

struct Override<'a> {
  instance_buffer: AbstractReadonlyStorageBuffer<[NodeStorage]>,
  instance_meta: AbstractReadonlyStorageBuffer<[InstanceMetaData]>,
  model_to_instance: AbstractReadonlyStorageBuffer<[u32]>,
  internal: Box<dyn RenderComponent + 'a>,
}

impl<'a> ShaderHashProvider for Override<'a> {
  shader_hash_type_id! {Override<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.internal.hash_pipeline_with_type_info(hasher);
  }
}

impl<'a> GraphicsShaderProvider for Override<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      let instance_buffer = binding.bind_by(&self.instance_buffer);
      let instance_meta = binding.bind_by(&self.instance_meta);
      let model_to_instance = binding.bind_by(&self.model_to_instance);

      let vertex_index = vertex.query::<VertexIndex>();
      let instance_scene_model_id = vertex.query::<LogicalRenderEntityId>();
      let instance_model_id = model_to_instance.index(instance_scene_model_id).load();

      let instance_meta = instance_meta.index(instance_model_id).load().expand();
      vertex.register::<LogicalRenderEntityId>(instance_meta.origin_model); // todo, the std model has issue

      let origin_object_vertex_count = instance_meta.instance_vertices_count;

      let real_instance_index = vertex_index / origin_object_vertex_count;
      let real_vertex_index = vertex_index % origin_object_vertex_count;

      vertex.register::<VertexIndex>(real_vertex_index);

      let instance_transform = instance_buffer
        .index(instance_meta.instance_offset + real_instance_index)
        .load();

      // todo, we should put this mul at host side to save cost
      register_or_compose_world_related_info(vertex, instance_meta.owned_transform.expand());

      register_or_compose_world_related_info(vertex, instance_transform.expand());
    });

    self.internal.build(builder);
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.internal.post_build(builder);
  }
}

impl<'a> ShaderPassBuilder for Override<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.instance_buffer);
    ctx.binding.bind(&self.instance_meta);
    ctx.binding.bind(&self.model_to_instance);

    self.internal.setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.internal.post_setup_pass(ctx);
  }
}
