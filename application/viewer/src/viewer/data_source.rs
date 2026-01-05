use crate::*;

pub fn viewer_mesh_input<Cx>(cx: &mut Cx) -> UseResult<AttributesMeshDataChangeInput>
where
  Cx: DBHookCxLike,
{
  //   struct DBMeshInput;
  //   impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for DBMeshInput {
  //     share_provider_hash_type_id! {}
  //     type Result = AttributesMeshDataChangeInput;
  //     fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
  //       attribute_mesh_input(cx)
  //     }
  //   }

  //   let (cx, scheduler) = cx
  //     .use_plain_state::<Arc<RwLock<NoScheduleScheduler<u32, Arc<GPUBufferImage>>>>>(|| {
  //       let source = InMemoryUriDataSource::new(alloc_global_res_id());
  //       let scheduler = NoScheduleScheduler::new(Box::new(source));
  //       Arc::new(RwLock::new(scheduler))
  //     });

  //   let iter = [].into_iter(); // todo
  //   use_maybe_uri_data_changes(cx, DBMeshInput, scheduler, Box::new(iter))

  attribute_mesh_input(cx)
}

// todo, share scheduler
// todo, LinearBatchChanges<u32, Option<GPUBufferImage>>'s iter will cause excessive clone
// so we use Arc, but we should use DataChangeRef trait
pub fn viewer_texture_input(
  cx: &mut QueryGPUHookCx<'_>,
) -> UseResult<Arc<LinearBatchChanges<u32, Option<Arc<GPUBufferImage>>>>> {
  let (cx, scheduler) = cx
    .use_plain_state::<Arc<RwLock<NoScheduleScheduler<u32, Arc<GPUBufferImage>>>>>(|| {
      let source = InMemoryUriDataSource::new(alloc_global_res_id());
      let scheduler = NoScheduleScheduler::new(Box::new(source));
      Arc::new(RwLock::new(scheduler))
    });

  let iter = get_db_view_no_generation_check::<SceneTexture2dEntityDirectContent>()
    .iter_static_life()
    .filter_map(|(k, v)| {
      let v = v?;
      let v = match v.ptr.as_ref() {
        MaybeUriData::Uri(_) => None,
        MaybeUriData::Living(v) => Some(v),
      }?;
      Some((k, v.clone()))
    });

  struct DBTextureInput;
  impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for DBTextureInput {
    share_provider_hash_type_id! {}
    type Result =
      Arc<FastChangeCollector<<SceneTexture2dEntityDirectContent as ComponentSemantic>::Data>>;
    fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
      cx.use_changes::<SceneTexture2dEntityDirectContent>()
    }
  }

  use_maybe_uri_data_changes(cx, DBTextureInput, scheduler, Box::new(iter))
}
