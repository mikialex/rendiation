use crate::*;

impl MeshLODGraphRenderer {
  /// expand a device list of scene model into a device list of meshlet with culling and lod logic
  pub fn expand(
    &self,
    scene_models: &DeviceSceneModelRenderSubBatch,
    cx: &mut DeviceParallelComputeCtx,
  ) -> MeshletBatchDrawData {
    let scene_models = scene_models.scene_models.execute_and_expose(cx);

    let mut hasher = shader_hasher_from_marker_ty!(MeshLODMeshExpand);
    scene_models.hash_pipeline(&mut hasher);
    let pipeline = cx
      .gpu
      .device
      .get_or_cache_create_compute_pipeline_by(hasher, |mut ctx| {
        //
        ctx
      });
    //

    todo!()
  }
}

pub struct MeshletBatchDrawData {
  meshlets: StorageBufferDataView<[MeshletDrawCommand]>,
  command: DrawCommand,
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
struct MeshletDrawCommand {
  pub meshlet_id: u32,
  pub scene_model_id: u32,
}
