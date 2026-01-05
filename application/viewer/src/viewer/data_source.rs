use rendiation_uri_scheduler::*;

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

  // let (cx, scheduler) = cx
  //   .use_plain_state::<Arc<RwLock<NoScheduleScheduler<u32, Arc<GPUBufferImage>>>>>(|| {
  //     let source = InMemoryUriDataSource::new(alloc_global_res_id());
  //     let scheduler = NoScheduleScheduler::new(Box::new(source));
  //     Arc::new(RwLock::new(scheduler))
  //   });

  //   let iter = [].into_iter(); // todo
  //   use_maybe_uri_data_changes(cx, DBMeshInput, scheduler, Box::new(iter))

  attribute_mesh_input(cx)
}

// struct MaybeUriMesh {}

// fn read_maybe_uri_mesh(
//   reader: &AttributesMeshReader,
// ) -> MaybeUriData<AttributesMesh, MaybeUriMesh> {
//   todo!()
// }

// todo, share scheduler
// todo, LinearBatchChanges<u32, Option<GPUBufferImage>>'s iter will cause excessive clone
// so we use Arc, but we should use DataChangeRef trait
pub fn viewer_texture_input(
  cx: &mut QueryGPUHookCx<'_>,
) -> UseResult<Arc<LinearBatchChanges<u32, Option<Arc<GPUBufferImage>>>>> {
  let (cx, scheduler) = cx
    .use_plain_state::<Arc<RwLock<NoScheduleScheduler<u32, Arc<GPUBufferImage>, Arc<String>>>>>(
      || {
        let mut source = InMemoryUriDataSource::<Arc<GPUBufferImage>>::new(alloc_global_res_id());
        let load_impl = move |uri: &Arc<String>| {
          Box::new(source.request_uri_data_load(uri.as_str()))
            as Box<dyn Future<Output = Option<Arc<GPUBufferImage>>> + Send + Sync + Unpin>
        };

        let scheduler = NoScheduleScheduler::new(Box::new(load_impl));
        Arc::new(RwLock::new(scheduler))
      },
    );

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

  struct DBTextureUriInput;
  impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for DBTextureUriInput {
    share_provider_hash_type_id! {}
    type Result = Arc<LinearBatchChanges<u32, MaybeUriData<Arc<GPUBufferImage>>>>;
    fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
      cx.use_changes::<SceneTexture2dEntityDirectContent>()
        .map(|changes| {
          changes
            .collective_filter_map(|v| v.map(|v| (*v).clone()))
            .materialize()
        })
    }
  }

  use_uri_data_changes(cx, DBTextureUriInput, scheduler, Box::new(iter))
}
