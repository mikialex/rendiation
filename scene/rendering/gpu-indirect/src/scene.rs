use crate::*;

pub struct IndirectSceneRenderer {
  pub texture_system: GPUTextureBindingSystem,
  pub renderer: Box<dyn IndirectBatchSceneModelRenderer>,
  pub reversed_depth: bool,
  pub using_host_driven_indirect_draw: bool,
  pub model_error_state: SceneModelErrorRecorder,
  pub gpu: GPU,
}

#[derive(Debug)]
struct MissingIndirectGroupKeyError;

impl IndirectSceneRenderer {
  pub fn classify_draws(
    &self,
    iter: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
  ) -> FastHashMap<u64, Vec<EntityHandle<SceneModelEntity>>> {
    let mut classifier = FastHashMap::default();

    for sm in iter {
      let mut hasher = PipelineHasher::default();
      let re = self
        .renderer
        .hash_shader_group_key_with_self_type_info(sm, &mut hasher)
        .ok_or(MissingIndirectGroupKeyError);

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
    let mut list_info = Vec::with_capacity(classified.len());
    let mut real_lengths = Vec::with_capacity(classified.len());

    let limits = &self.gpu.info.supported_limits;
    let align = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    fn round_up(value: u32, alignment: u32) -> u32 {
      (value + alignment - 1) / alignment * alignment
    }

    let mut impl_select_ids = Vec::with_capacity(classified.len());
    for (_, list) in &classified {
      let real_len = list.len() as u32;
      let padded_len = round_up(real_len, align);
      let offset = models.len() as u32;
      impl_select_ids.push(*list.first().unwrap());
      real_lengths.push(real_len);
      list_info.push(SubListHostInfo {
        capacity: padded_len,
        offset,
      });
      models.extend(list.iter().map(|sm| sm.alloc_index()));
      // Pad the pool to the aligned capacity so that each sub-list's region
      // starts at its capacity-based offset and buffer views satisfy alignment.
      let padding = (padded_len - real_len) as usize;
      models.resize(models.len() + padding, 0);
    }

    let scene_model_id_pool = create_gpu_readonly_storage(models.as_slice(), &self.gpu);
    let sub_list_ranges_gpu = prepare_gpu_sub_list_ranges(&list_info, real_lengths.as_slice());
    let sub_list_ranges = create_gpu_readonly_storage(sub_list_ranges_gpu.as_slice(), &self.gpu);
    let sum_all_count_host = model_counts as u32;
    let sum_all_count = create_gpu_readonly_storage(&sum_all_count_host, &self.gpu);

    let draw_list = DeviceDrawList {
      scene_model_id_pool,
      dispatch_info: MultiRangeDispatchInfo {
        sub_list_infos: list_info,
        sub_list_ranges,
        sum_all_count,
        sum_all_count_host,
      },
    };

    DeviceSceneModelDrawList {
      draw_list,
      impl_select_ids,
    }
    .into()
  }
}

pub trait IndirectDrawProviderCreator {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64>;

  /// the sub_lists's impl_select_id's impl_distinguish_key must be all same for this list
  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    id: RawEntityHandle,
  ) -> Option<Vec<Box<dyn IndirectDrawProvider>>>;
}

pub trait DrawCommandBuilderCreator {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder>;
}

impl DrawCommandBuilderCreator for IndirectSceneRenderer {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    self.renderer.make_draw_command_builder(id)
  }
}

impl IndirectDrawProviderCreator for IndirectSceneRenderer {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    self.renderer.get_impl_distinguish_key_by_impl_select_id(id)
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    id: RawEntityHandle,
  ) -> Option<Vec<Box<dyn IndirectDrawProvider>>> {
    self
      .renderer
      .use_create_or_update_indirect_draw_providers(cx, list, id)
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

  fn make_scene_batch_pass_content<'a>(
    &'a self,
    list: SceneModelRenderBatch,
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let device_list = match list {
      SceneModelRenderBatch::Device(batch) => batch,
      SceneModelRenderBatch::Host(batch) => {
        if self.using_host_driven_indirect_draw {
          return ctx.scope(|ctx| {
            self.process_host_driven_indirect_draws(batch.as_ref(), ctx, camera, pass)
          });
        }
        self.create_batch_from_iter(&mut batch.iter_scene_models())
      }
    };

    let Some(device_list) = device_list else {
      return Box::new(IndirectScenePassContent {
        renderer: self,
        content: Vec::new(),
        pass,
        camera,
        reversed_depth: self.reversed_depth,
      });
    };

    ctx.scope(|ctx| {
      let mut classified: FastHashMap<u64, (Vec<usize>, Vec<EntityHandle<SceneModelEntity>>)> =
        FastHashMap::default();
      let mut mappings = Vec::new();

      assert_eq!(
        device_list.draw_list.dispatch_info.sub_list_infos.len(),
        device_list.impl_select_ids.len()
      );
      for (i, impl_select_id) in device_list.impl_select_ids.iter().enumerate() {
        if let Some(impl_key) =
          self.get_impl_distinguish_key_by_impl_select_id(impl_select_id.into_raw())
        {
          let (list, list_ids) = classified.entry(impl_key).or_default();
          let idx = list.len();
          list.push(i);
          list_ids.push(*impl_select_id);
          mappings.push((impl_key, idx, impl_select_id));
        } else {
          log::error!("unable to find impl key");
        }
      }

      let mut indirect_draw_providers: FastHashMap<
        u64,
        FastHashMap<usize, Box<dyn IndirectDrawProvider>>,
      > = Default::default();

      ctx.next_key_scope_root();
      for (impl_key, (selected_sub_list, impl_select_ids)) in &classified {
        ctx.keyed_scope(impl_key, |ctx| {
          let dispatch_info = ctx.access_parallel_compute(|ctx| {
            compute_selected_sub_list_dispatch_info(ctx, &device_list.draw_list, selected_sub_list)
          });
          let device_list_sub_list = DeviceDrawList {
            scene_model_id_pool: device_list.draw_list.scene_model_id_pool.clone(),
            dispatch_info,
          };

          ctx.access_parallel_compute(|cx| {
            if let Some(result) = self.use_create_or_update_indirect_draw_providers(
              cx,
              &device_list_sub_list,
              impl_select_ids[0].into_raw(),
            ) {
              // using map is to avoid IndirectDrawProvider impl clone
              let map = result.into_iter().enumerate().collect();
              indirect_draw_providers.insert(*impl_key, map);
            } else {
              log::error!("unable to create indirect draw provider");
            }
          })
        });
      }

      let content = mappings
        .iter()
        .filter_map(|(impl_id, index, impl_select_sm_id)| {
          let provider = indirect_draw_providers.get_mut(impl_id)?.remove(index)?;
          (provider, **impl_select_sm_id).into()
        })
        .collect();

      Box::new(IndirectScenePassContent {
        renderer: self,
        content,
        pass,
        camera,
        reversed_depth: self.reversed_depth,
      })
    })
  }
}

fn compute_selected_sub_list_dispatch_info(
  cx: &mut DeviceParallelComputeCtx,
  input: &DeviceDrawList,
  pick_list: &[usize],
) -> MultiRangeDispatchInfo {
  let pick_count = pick_list.len();
  debug_assert!(pick_count > 0, "pick_list should never be empty");

  // Collect host-side sub_list_infos for the selected sub-lists.
  // Offsets are recalculated as compact cumulative capacities for correct
  // output buffer layout downstream (prepare_gpu_sub_list_out_ranges and
  // MIDC downgrade command-pool slicing). The GPU-side sub_list_ranges.x
  // preserves the original pool offset for correct scene_model_id_pool indexing.
  let mut compact_offset = 0u32;
  let selected_infos: Vec<SubListHostInfo> = pick_list
    .iter()
    .map(|&i| {
      let info = &input.dispatch_info.sub_list_infos[i];
      let new_info = SubListHostInfo {
        capacity: info.capacity,
        offset: compact_offset,
      };
      compact_offset += info.capacity;
      new_info
    })
    .collect();

  // sum_all_count_host is set to the sum of capacities (upper bound);
  // the GPU writes the real total into sum_all_count at runtime.
  let sum_capacity_host: u32 = selected_infos.iter().map(|info| info.capacity).sum();

  // Upload pick_list indices to the GPU.
  let pick_list_u32: Vec<u32> = pick_list.iter().map(|&i| i as u32).collect();
  let pick_list_buffer = create_gpu_readonly_storage(pick_list_u32.as_slice(), &cx.gpu);

  // Output ranges buffer — one Vec4<u32> per selected sub-list.
  let output_ranges = StorageBufferDataView::create_by_with_extra_usage(
    cx.gpu.device.as_ref(),
    StorageBufferInit::<[Vec4<u32>]>::from(ZeroedArrayByArrayLength(pick_count)),
    BufferUsages::INDIRECT,
  );

  // Output sum_all_count — GPU writes the real total count.
  let output_sum_all: StorageBufferDataView<u32> = create_gpu_read_write_storage(
    StorageBufferSizedZeroed::<u32>::default(),
    cx.gpu.device.as_ref(),
  );

  cx.record_pass(|pass, device| {
    let hasher = shader_hasher_from_marker_ty!(ComputeSelectedSubListDispatchInfo);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);

      let input_ranges = builder.bind_by(&input.dispatch_info.sub_list_ranges);
      let pick_list_storage = builder.bind_by(&pick_list_buffer);
      let out_ranges = builder.bind_by(&output_ranges);
      let out_sum_all: ShaderPtrOf<u32> = builder.bind_by(&output_sum_all);

      let total = val(0u32).make_local_var();
      let prefix = val(0u32).make_local_var();

      pick_list_storage
        .array_length()
        .into_shader_iter()
        .for_each(|i, _| {
          let src_idx = pick_list_storage.index(i).load();
          let src = input_ranges.index(src_idx).load();
          let count = src.y();
          let offset = src.x();

          out_ranges
            .index(i)
            .store(vec4_node((offset, count, prefix.load(), val(0u32))));

          prefix.store(prefix.load() + count);
          total.store(total.load() + count);
        });

      out_sum_all.store(total.load());

      builder
    });

    BindingBuilder::default()
      .with_bind(&input.dispatch_info.sub_list_ranges)
      .with_bind(&pick_list_buffer)
      .with_bind(&output_ranges)
      .with_bind(&output_sum_all)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(1, 1, 1);
  });

  MultiRangeDispatchInfo {
    sub_list_ranges: output_ranges.into_readonly_view(),
    sum_all_count: output_sum_all.into_readonly_view(),
    sub_list_infos: selected_infos,
    sum_all_count_host: sum_capacity_host,
  }
}

pub struct IndirectScenePassContent<'a> {
  pub renderer: &'a IndirectSceneRenderer,
  pub content: Vec<(
    Box<dyn IndirectDrawProvider>,
    EntityHandle<SceneModelEntity>,
  )>,

  pub pass: &'a dyn RenderComponent,
  pub camera: &'a dyn RenderComponent,
  pub reversed_depth: bool,
}

impl PassContent for IndirectScenePassContent<'_> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    let base = default_dispatcher(cx, self.reversed_depth).disable_auto_write();
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    for (content, any_scene_model) in &self.content {
      self.renderer.renderer.render_indirect_batch_models(
        content.as_ref(),
        *any_scene_model,
        &self.camera,
        &self.renderer.texture_system,
        &p,
        &mut cx.ctx,
      );
    }
  }
}
