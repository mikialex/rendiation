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
    .declare_sparse_foreign_key::<StandardModelRefLodGraphMeshEntity>();
}

pub fn use_mesh_lod_graph_scene_renderer(
  cx: &mut QueryGPUHookCx,
) -> Option<MeshLODGraphSceneRenderer> {
  let internal = use_mesh_lod_graph_renderer(cx);
  let world_transform = use_scene_model_device_world_transform(cx);

  let (cx, lod_decider) =
    cx.use_gpu_init(|gpu, _| create_uniform(LODDecider::zeroed(), &gpu.device));

  cx.when_render(|| MeshLODGraphSceneRenderer {
    mesh_ty_checker: global_database().read_foreign_key::<StandardModelRefLodGraphMeshEntity>(),
    world_transform: world_transform.unwrap(),
    lod_decider: lod_decider.clone(),
    internal,
  })
}

#[derive(Clone)]
pub struct MeshLODGraphSceneRenderer {
  mesh_ty_checker: ForeignKeyReadView<StandardModelRefLodGraphMeshEntity>,
  world_transform: DrawUnitWorldTransformProviderDefaultImpl,
  lod_decider: UniformBufferDataView<LODDecider>,
  internal: MeshLODGraphRendererShared,
}

impl MeshLODGraphSceneRenderer {
  pub fn setup_lod_decider(
    &self,
    gpu: &GPU,
    camera_perspective_mat: Mat4<f32>,
    camera_world: Mat4<f64>,
    view_size: Vec2<f32>,
  ) {
    let (near, _) = camera_perspective_mat.get_near_far_assume_perspective();
    let position = into_hpt(camera_world.position());

    let mut lod_decider = LODDecider::zeroed();
    lod_decider.camera_projection = camera_perspective_mat;
    lod_decider.view_size = view_size;
    lod_decider.camera_near = near;
    lod_decider.camera_world_position = position.into_uniform();
    self.lod_decider.write_at(&gpu.queue, &lod_decider, 0);
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
    self.internal.read().create_mesh_accessor().into()
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
        .read()
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
