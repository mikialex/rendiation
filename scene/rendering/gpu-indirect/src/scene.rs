use crate::*;

pub struct IndirectSceneRenderer {
  pub texture_system: GPUTextureBindingSystem,
  pub renderer: Box<dyn IndirectBatchSceneModelRenderer>,
  pub reversed_depth: bool,
  pub using_host_driven_indirect_draw: bool,
  pub model_error_state: SceneModelErrorRecorder,
  pub gpu: GPU,
}

impl IndirectSceneRenderer {
  pub fn classify_draws(
    &self,
    iter: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  ) -> FastHashMap<u64, Vec<EntityHandle<SceneModelEntity>>> {
    let mut classifier = FastHashMap::default();

    for sm in iter {
      let mut hasher = PipelineHasher::default();
      let re = self.renderer.hash_shader_group_key(sm, &mut hasher);

      self.model_error_state.report_and_filter_error(sm, &re);

      if re.is_ok() {
        let shader_hash = hasher.finish();
        let list = classifier.entry(shader_hash).or_insert_with(Vec::new);
        list.push(sm);
      }
    }

    classifier
  }
}

impl SceneDeviceBatchDirectCreator for IndirectSceneRenderer {
  // todo, use hook cache
  fn create_batch_from_iter(
    &self,
    iter: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  ) -> Option<DeviceSceneModelDrawList> {
    let classified = self.classify_draws(iter);

    if classified.is_empty() {
      return None;
    }

    let model_counts: usize = classified.iter().map(|(_, list)| list.len()).sum();
    let mut models = Vec::with_capacity(model_counts);
    let mut host_capacity_ranges = Vec::with_capacity(classified.len());
    let mut real_lengths = Vec::with_capacity(classified.len());

    let limits = &self.gpu.info.supported_limits;
    let align = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    fn round_up(value: u32, alignment: u32) -> u32 {
      value.div_ceil(alignment) * alignment
    }

    let mut impl_select_ids = Vec::with_capacity(classified.len());
    for (_, list) in &classified {
      let real_len = list.len() as u32;
      assert!(real_len > 0);
      let padded_len = round_up(real_len, align);
      let offset = models.len() as u32;
      impl_select_ids.push(*list.first().unwrap());
      real_lengths.push(real_len);
      host_capacity_ranges.push(CapacityRange {
        capacity: padded_len,
        offset,
      });
      models.extend(list.iter().map(|sm| sm.alloc_index()));
      // Pad the pool to the aligned capacity so that each sub-list's region
      // starts at its capacity-based offset and buffer views satisfy alignment.
      let padding = (padded_len - real_len) as usize;
      models.resize(models.len() + padding, 0);
    }

    let scene_model_id_pool = create_gpu_readonly_storage(
      models.as_slice(),
      &self.gpu,
      "scene_model_id_pool from batch-direct",
    );
    let sub_list_ranges =
      prepare_gpu_sub_list_ranges(&host_capacity_ranges, real_lengths.as_slice());
    let device_ranges = DeviceMultiRangeDispatchInfo::new(&self.gpu, sub_list_ranges.as_slice());

    let draw_list = DeviceDrawList {
      id_pool: scene_model_id_pool,
      dispatch_info: MultiRangeDispatchInfo {
        host_capacity_ranges,
        device_ranges,
        // the real count is known, it's a tighter bound than the capacity sum
        total_capacity: model_counts as u32,
      },
    };

    DeviceSceneModelDrawList {
      draw_list,
      impl_select_ids,
    }
    .into()
  }
}

impl SceneRenderer for IndirectSceneRenderer {
  fn indirect_batch_direct_creator(&self) -> Option<&dyn SceneDeviceBatchDirectCreator> {
    if self.using_host_driven_indirect_draw {
      None
    } else {
      Some(self)
    }
  }

  fn use_make_scene_batch_pass_content<'a>(
    &'a self,
    list: SceneModelRenderBatch,
    ctx: &mut FrameCtx,
  ) -> Box<dyn SceneRendererPassContentSource + 'a> {
    ctx.next_scope_index();
    let device_list = match list {
      SceneModelRenderBatch::Device(batch) => batch,
      SceneModelRenderBatch::Host(batch) => {
        if self.using_host_driven_indirect_draw {
          return ctx.scope(|ctx| self.process_host_driven_indirect_draws(batch.as_ref(), ctx));
        }
        self.create_batch_from_iter(&mut batch.iter_scene_models())
      }
    };

    let Some(device_list) = device_list else {
      return Box::new(IndirectScenePassContentSource {
        renderer: self,
        content: Vec::new(),
      });
    };

    ctx.scope(|ctx| {
      let mut classified: FastHashMap<u64, (Vec<usize>, Vec<EntityHandle<SceneModelEntity>>)> =
        FastHashMap::default();
      let mut mappings = Vec::new();

      assert_eq!(
        device_list
          .draw_list
          .dispatch_info
          .host_capacity_ranges
          .len(),
        device_list.impl_select_ids.len()
      );
      for (i, impl_select_id) in device_list.impl_select_ids.iter().enumerate() {
        match self.renderer.impl_distinguish_key(*impl_select_id) {
          Ok(impl_key) => {
            let (list, list_ids) = classified.entry(impl_key).or_default();
            let idx = list.len();
            list.push(i);
            list_ids.push(*impl_select_id);
            mappings.push((impl_key, idx, impl_select_id));
          }
          Err(err) => log::error!("unable to find impl key: {err}"),
        }
      }

      let mut indirect_draw_providers: FastHashMap<
        u64,
        FastHashMap<usize, Box<dyn IndirectDrawProvider>>,
      > = Default::default();

      ctx.access_parallel_compute(|ctx| {
        ctx.next_scope_index();
        for (impl_key, (selected_sub_list, impl_select_ids)) in &classified {
          // the impl key contains the impl type identity, so the hook states of different impls
          // are isolated by this scope
          ctx.keyed_scope(impl_key, |ctx| {
            let dispatch_info = use_compute_selected_sub_list_dispatch_info(
              ctx,
              &device_list.draw_list,
              selected_sub_list,
            );
            let device_list_sub_list = DeviceDrawList {
              id_pool: device_list.draw_list.id_pool.clone(),
              dispatch_info,
            };

            match self.renderer.use_create_or_update_indirect_draw_providers(
              ctx,
              &device_list_sub_list,
              impl_select_ids[0],
            ) {
              Ok(result) => {
                // using map is to avoid IndirectDrawProvider impl clone
                let map = result.into_iter().enumerate().collect();
                indirect_draw_providers.insert(*impl_key, map);
              }
              Err(err) => log::error!("unable to create indirect draw provider: {err}"),
            }
          });
        }
      });

      let content = mappings
        .iter()
        .filter_map(|(impl_id, index, impl_select_sm_id)| {
          let provider = indirect_draw_providers.get_mut(impl_id)?.remove(index)?;
          (provider, **impl_select_sm_id).into()
        })
        .collect();

      Box::new(IndirectScenePassContentSource {
        renderer: self,
        content,
      })
    })
  }
}

/// Select the sub lists from the input list, the selected sub lists share the input's id pool,
/// so the pool regions(the host capacity ranges) are kept, only the count prefix sum is
/// recomputed.
fn use_compute_selected_sub_list_dispatch_info(
  cx: &mut DeviceParallelComputeCtx,
  input: &DeviceDrawList,
  pick_list: &[usize],
) -> MultiRangeDispatchInfo {
  let pick_count = pick_list.len();
  debug_assert!(pick_count > 0, "pick_list should never be empty");

  let selected_infos: Vec<_> = pick_list
    .iter()
    .map(|&i| input.dispatch_info.host_capacity_ranges[i])
    .collect();

  // the capacity sum is the upper bound, the GPU writes the real total into sum_all_count
  let total_capacity = selected_infos.iter().map(|info| info.capacity).sum();

  let pick_list_u32: Vec<_> = pick_list.iter().map(|&i| i as u32).collect();
  let pick_list_buffer =
    cx.use_storage_buffer_array_with_host_data_queue_write_sync(&pick_list_u32, "pick_list");

  let output_ranges = cx.use_rw_storage_buffer_array_impl::<StorageSubListRangeInfo>(
    pick_count,
    "output_ranges",
    BufferUsages::INDIRECT,
  );

  let output_sum_all =
    cx.use_rw_storage_buffer_impl(&0_u32, "output_sum_all", BufferUsages::empty());

  cx.record_pass(|pass, device| {
    let hasher = shader_hasher_from_marker_ty!(ComputeSelectedSubListDispatchInfo);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);

      let input_ranges = builder.bind_by(&input.dispatch_info.device_ranges.sub_list_ranges);
      let pick_list_storage = builder.bind_by(&pick_list_buffer);
      let output_ranges = builder.bind_by(&output_ranges);
      let output_sum_all = builder.bind_by(&output_sum_all);

      let prefix = val(0u32).make_local_var();

      // todo, for simplicity this dispatch is not parallel
      pick_list_storage
        .array_length()
        .into_shader_iter()
        .for_each(|i, _| {
          let src_idx = pick_list_storage.index(i).load();
          let src = input_ranges.index(src_idx).load();
          let src = src.expand();
          let count = src.count;

          output_ranges.index(i).store(
            ENode::<StorageSubListRangeInfo> {
              offset: src.offset,
              count,
              count_prefix_sum: prefix.load(),
            }
            .construct(),
          );

          prefix.store(prefix.load() + count);
        });

      output_sum_all.store(prefix.load());

      builder
    });

    BindingBuilder::default()
      .with_bind(&input.dispatch_info.device_ranges.sub_list_ranges)
      .with_bind(&pick_list_buffer)
      .with_bind(&output_ranges)
      .with_bind(&output_sum_all)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(1, 1, 1);
  });

  MultiRangeDispatchInfo {
    device_ranges: DeviceMultiRangeDispatchInfo {
      sub_list_ranges: output_ranges.into_readonly_view(),
      sum_all_count: output_sum_all.into_readonly_view(),
    },
    host_capacity_ranges: selected_infos,
    total_capacity,
  }
}

pub struct IndirectScenePassContentSource<'a> {
  pub renderer: &'a IndirectSceneRenderer,
  pub content: Vec<(
    Box<dyn IndirectDrawProvider>,
    EntityHandle<SceneModelEntity>,
  )>,
}

impl SceneRendererPassContentSource for IndirectScenePassContentSource<'_> {
  fn as_pass_content<'a>(
    &'a self,
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
  ) -> Box<dyn PassContent + 'a> {
    Box::new(IndirectScenePassContent {
      renderer: self.renderer,
      content: &self.content,
      pass,
      camera,
    })
  }
}

pub struct IndirectScenePassContent<'a> {
  pub renderer: &'a IndirectSceneRenderer,
  pub content: &'a [(
    Box<dyn IndirectDrawProvider>,
    EntityHandle<SceneModelEntity>,
  )],

  pub pass: &'a dyn RenderComponent,
  pub camera: &'a dyn RenderComponent,
}

impl PassContent for IndirectScenePassContent<'_> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    let base = default_dispatcher(cx, self.renderer.reversed_depth).disable_auto_write();
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    for (content, any_scene_model) in self.content {
      let re = self.renderer.renderer.render_indirect_batch_models(
        content.as_ref(),
        *any_scene_model,
        &self.camera,
        &self.renderer.texture_system,
        &p,
        &mut cx.ctx,
      );
      self
        .renderer
        .model_error_state
        .report_and_filter_error(*any_scene_model, &re);
    }
  }
}
