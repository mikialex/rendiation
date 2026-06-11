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

    let mut impl_select_ids = Vec::with_capacity(classified.len());
    for (_, list) in &classified {
      let offset = models.len() as u32;
      impl_select_ids.push(*list.first().unwrap());
      list_info.push(SubListHostInfo {
        capacity: list.len() as u32,
        offset,
      });
      models.extend(list.iter().map(|sm| sm.alloc_index()));
    }

    let scene_model_id_pool = create_gpu_readonly_storage(models.as_slice(), &self.gpu);
    let sub_list_ranges_gpu = compute_gpu_sub_list_ranges(&list_info);
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
      let mut classified: FastHashMap<
        u64,
        (Vec<SubListHostInfo>, Vec<EntityHandle<SceneModelEntity>>),
      > = FastHashMap::default();
      let mut mappings = Vec::new();

      let sub_list_infos_iter = device_list.draw_list.dispatch_info.sub_list_infos.iter();
      for (info, impl_select_id) in sub_list_infos_iter.zip(device_list.impl_select_ids.iter()) {
        if let Some(impl_key) =
          self.get_impl_distinguish_key_by_impl_select_id(impl_select_id.into_raw())
        {
          let (list, list_ids) = classified.entry(impl_key).or_default();
          let idx = list.len();
          list.push(info.clone());
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
          let (ctx, target_state) = ctx.use_plain_state_default::<Option<DeviceDrawList>>();
          let device_list_sub_list = device_list.draw_list.create_or_update_compact_write_target(
            &ctx.gpu,
            target_state,
            &selected_sub_list,
          );

          ctx.access_parallel_compute(|cx| {
            if let Some(result) = self.use_create_or_update_indirect_draw_providers(
              cx,
              device_list_sub_list,
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
