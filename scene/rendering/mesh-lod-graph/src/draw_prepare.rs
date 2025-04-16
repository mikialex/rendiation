use crate::*;

struct MeshLODExpander {
  lod_decider: UniformBufferDataView<LODDecider>,
  meshlet_meta: StorageBufferDataView<[MeshletMetaData]>,
  scene_model_meshlet_range: StorageBufferDataView<[Vec2<u32>]>,
}

impl MeshLODExpander {
  /// expand a device list of scene model into a device list of meshlet with culling and lod logic
  pub fn expand(
    &self,
    scene_models: &DeviceSceneModelRenderSubBatch,
    cx: &mut DeviceParallelComputeCtx,
    max_draw_count: u32,
  ) -> MeshletBatchDrawData {
    let scene_models = scene_models.scene_models.execute_and_expose(cx);

    let device = &cx.gpu.device;
    let init = ZeroedArrayByArrayLength(max_draw_count as usize);
    let bumper: StorageBufferDataView<DeviceAtomic<u32>> = todo!();
    let meshlet_idx_output = create_gpu_read_write_storage::<[u32]>(init, device);
    let draw_command_output = create_gpu_read_write_storage::<[DrawIndexedIndirect]>(init, device);

    cx.record_pass(|pass, device| {
      let mut hasher = shader_hasher_from_marker_ty!(MeshLODMeshExpand);
      scene_models.hash_pipeline(&mut hasher);
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut ctx| {
        //
        let bumper = ctx.bind_by(&bumper);
        let meshlet_idx_output = ctx.bind_by(&meshlet_idx_output);
        let draw_command_output = ctx.bind_by(&draw_command_output);
        let meshlet_meta = ctx.bind_by(&self.meshlet_meta);
        let scene_model_meshlet_range = ctx.bind_by(&self.scene_model_meshlet_range);
        let lod_decider = ctx.bind_by(&self.lod_decider);

        let scene_model = scene_models.build_shader(&mut ctx);
        let (scene_model, valid) = scene_model.invocation_logic(ctx.global_invocation_id());
        if_by(valid, || {
          let range = scene_model_meshlet_range.index(scene_model).load();
          // range.i
        });

        ctx
      });

      let mut bb = BindingBuilder::default()
        .with_bind(&bumper)
        .with_bind(&meshlet_idx_output)
        .with_bind(&draw_command_output)
        .with_bind(&self.meshlet_meta)
        .with_bind(&self.scene_model_meshlet_range)
        .with_bind(&self.lod_decider);

      scene_models.bind_input(&mut bb);

      bb.setup_compute_pass(pass, device, &pipeline);
      pass.dispatch_workgroups(todo!(), 1, 1);
    });

    MeshletBatchDrawData {
      meshlets_idx: meshlet_idx_output,
      command: DrawCommand::MultiIndirectCount {
        indexed: true,
        indirect_buffer: todo!(),
        indirect_count: todo!(),
        max_count: max_draw_count,
      },
    }
  }
}
