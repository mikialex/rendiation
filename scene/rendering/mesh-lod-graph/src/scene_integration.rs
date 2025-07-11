use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;

use crate::*;

declare_foreign_key!(
  StandardModelRefLodGraphMeshEntity,
  StandardModelEntity,
  LODGraphMeshEntity
);
pub fn register_scene_mesh_lod_graph_data_model() {
  register_mesh_lod_graph_data_model();
  global_entity_of::<StandardModelEntity>()
    .declare_foreign_key::<StandardModelRefLodGraphMeshEntity>();
}

pub fn use_mesh_lod_graph_renderer(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<MeshLODGraphSceneRendererShared> {
  // ReactiveRangeAllocatePool::new();

  todo!()
}

pub type MeshLODGraphSceneRendererShared = Arc<RwLock<MeshLODGraphSceneRenderer>>;

pub struct MeshLODGraphSceneRenderer {
  mesh_ty_checker: ForeignKeyReadView<StandardModelRefLodGraphMeshEntity>,
  world_transform: DrawUnitWorldTransformProviderDefaultImpl,
  lod_decider: UniformBufferDataView<LODDecider>,
  internal: MeshLODGraphRenderer,
}

impl MeshLODGraphSceneRenderer {
  pub fn setup_lod_decider(
    &self,
    gpu: &GPU,
    camera_perspective_mat: Mat4<f32>,
    view_size: Vec2<f32>,
  ) {
    todo!()
  }
}

/// implementation notes:
///
/// - using current system, integrate into the indirect renderer
/// - the generate_indirect_draw_provider dependency can be setup outside(before) of the call, this is acceptable hack
/// - this integration should ok to reused the scene model level's culling and virtualization capability.
/// - to support per meshlet culling. we need
///   - create meshlet device batch container(mdbc) from DeviceSceneModelRenderSubBatch
///   - the mdbc may use DeviceSceneModelRenderSubBatch if we make batch item not stick to scene model
///   - implement other culling pipeline to support mdbc cull to mdbc
///   - implement mdbc to indirect draw provider, the current draw provider logic can be improved
///   - a way to extract all meshlet scene model DeviceSceneModelRenderSubBatch from  DeviceSceneModelRenderBatch
///   - add custom render procedure in top viewer integration. the explicit render procedure is unavoidable.
impl IndirectModelShapeRenderImpl for MeshLODGraphSceneRenderer {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    self.mesh_ty_checker.get(any_idx)?;
    self.internal.create_mesh_accessor().into()
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    _hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.mesh_ty_checker.get(any_id)?;
    Some(())
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    any_idx: EntityHandle<StandardModelEntity>,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    self.mesh_ty_checker.get(any_idx)?;

    let lod_decider = self.lod_decider.clone();
    let scene_model_matrix = &self.world_transform;
    let max_meshlet_count = 100000; // todo is this count enough?? how do we know this is not enough??

    ctx.access_parallel_compute(|ctx| {
      self
        .internal
        .prepare_draw(
          batch,
          ctx,
          lod_decider,
          scene_model_matrix,
          max_meshlet_count,
        )
        .into()
    })
  }

  fn make_draw_command_builder(
    &self,
    _any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    None
  }
}
