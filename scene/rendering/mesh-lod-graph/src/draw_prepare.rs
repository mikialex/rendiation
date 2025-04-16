use crate::*;

pub struct MeshLODExpander {
  pub lod_decider: UniformBufferDataView<LODDecider>,
  pub meshlet_metadata: StorageBufferReadonlyDataView<[MeshletMetaData]>,
  pub scene_model_meshlet_range: StorageBufferReadonlyDataView<[Vec2<u32>]>,
}

pub trait SceneModelWorldMatrixProvider: ShaderHashProvider {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn SceneModelWorldMatrixInvocationProvider>;
  fn bind(&self, cx: &mut BindingBuilder);
}

pub trait SceneModelWorldMatrixInvocationProvider {
  fn get_world_matrix(&self, id: Node<u32>) -> Node<Mat4<f32>>;
}

impl MeshLODExpander {
  /// expand a device list of scene model into a device list of meshlet with culling and lod logic
  pub fn expand(
    &self,
    scene_models: &DeviceSceneModelRenderSubBatch,
    scene_model_matrix: &dyn SceneModelWorldMatrixProvider,
    cx: &mut DeviceParallelComputeCtx,
    max_meshlet_count: u32,
  ) -> MeshletBatchDrawData {
    let scene_models = scene_models.scene_models.execute_and_expose(cx);
    let scene_model_size_indirect = scene_models.compute_work_size(cx);

    let device = &cx.gpu.device;
    let init = ZeroedArrayByArrayLength(max_meshlet_count as usize);
    let bumper = create_gpu_read_write_storage::<DeviceAtomic<u32>>(
      StorageBufferSizedZeroed::<DeviceAtomic<u32>>::default(),
      device,
    );
    let meshlet_idx_output = create_gpu_read_write_storage::<[u32]>(init, device);
    let scene_model_idx_output = create_gpu_read_write_storage::<[u32]>(init, device);
    let draw_command_output = create_gpu_read_write_storage::<[DrawIndexedIndirect]>(init, device);

    cx.record_pass(|pass, device| {
      let mut hasher = shader_hasher_from_marker_ty!(MeshLODMeshExpand);
      scene_models.hash_pipeline(&mut hasher);
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut ctx| {
        //
        let bumper = ctx.bind_by(&bumper);
        let meshlet_idx_output = ctx.bind_by(&meshlet_idx_output);
        let scene_model_idx_output = ctx.bind_by(&scene_model_idx_output);
        let draw_command_output = ctx.bind_by(&draw_command_output);
        let meshlet_metadata = ctx.bind_by(&self.meshlet_metadata);
        let scene_model_meshlet_range = ctx.bind_by(&self.scene_model_meshlet_range);
        let lod_decider = ctx.bind_by(&self.lod_decider).load().expand();
        let world_matrix_access = scene_model_matrix.create_invocation(&mut ctx.bindgroups);

        let scene_model = scene_models.build_shader(&mut ctx);
        let (scene_model, valid) = scene_model.invocation_logic(ctx.global_invocation_id());

        if_by(valid, || {
          let model_world_matrix = world_matrix_access.get_world_matrix(scene_model);

          let range = scene_model_meshlet_range.index(scene_model).load();
          range
            .into_shader_iter()
            .map(|meshlet_idx| (meshlet_metadata.index(meshlet_idx).load(), meshlet_idx))
            .for_each(|(meshlet, meshlet_idx), _| {
              let meshlet = meshlet.expand();
              let bound_pair = meshlet.bounds.expand();

              let is_lod_suitable = lod_decider.exact_lod_cut(
                bound_pair.self_lod,
                bound_pair.parent_lod,
                model_world_matrix,
              );

              if_by(is_lod_suitable, || {
                let write_idx = bumper.atomic_add(val(1));
                if_by(
                  write_idx.less_than(meshlet_idx_output.array_length()),
                  || {
                    meshlet_idx_output.index(write_idx).store(meshlet_idx);
                    scene_model_idx_output.index(write_idx).store(scene_model);

                    draw_command_output
                      .index(write_idx)
                      .store(ENode::<DrawIndexedIndirect> {
                        vertex_count: meshlet.index_count,
                        instance_count: val(1),
                        base_index: val(0), // accessed from meshlet metadata at vertex stage
                        vertex_offset: val(0),
                        base_instance: write_idx,
                      })
                  },
                );
              });
            });
        });

        ctx
      });

      let mut bb = BindingBuilder::default()
        .with_bind(&bumper)
        .with_bind(&meshlet_idx_output)
        .with_bind(&scene_model_idx_output)
        .with_bind(&draw_command_output)
        .with_bind(&self.meshlet_metadata)
        .with_bind(&self.scene_model_meshlet_range)
        .with_bind(&self.lod_decider);

      scene_model_matrix.bind(&mut bb);
      scene_models.bind_input(&mut bb);

      bb.setup_compute_pass(pass, device, &pipeline);
      pass.dispatch_workgroups_indirect_by_buffer_resource_view(&scene_model_size_indirect.0);
    });

    MeshletBatchDrawData {
      meshlets_idx: meshlet_idx_output.into_readonly_view(),
      scene_model_idx: scene_model_idx_output.into_readonly_view(),
      command: DrawCommand::MultiIndirectCount {
        indexed: true,
        indirect_buffer: draw_command_output.gpu,
        indirect_count: scene_model_size_indirect.1.gpu.clone(),
        max_count: max_meshlet_count,
      },
    }
  }
}
