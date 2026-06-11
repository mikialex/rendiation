use crate::*;

pub trait WriteSolidAttributeMesh {
  fn write_solid_attribute_mesh(&mut self, mesh: AttributesMesh) -> AttributesMeshEntities;
  fn write_solid_attribute_mesh_data_uri(
    &mut self,
    mesh: AttributesMesh,
    buffer_source: &mut dyn UriDataSourceDyn<Arc<Vec<u8>>>,
  ) -> AttributesMeshEntities;
}

impl WriteSolidAttributeMesh for SceneWriter {
  fn write_solid_attribute_mesh(&mut self, mesh: AttributesMesh) -> AttributesMeshEntities {
    let r = mesh.write(&mut self.mesh_writer, &mut self.buffer_writer);
    self
      .mesh_writer
      .mesh
      .write::<AttributeMeshIsSolid>(r.mesh, true);
    r
  }
  fn write_solid_attribute_mesh_data_uri(
    &mut self,
    mesh: AttributesMesh,
    buffer_source: &mut dyn UriDataSourceDyn<Arc<Vec<u8>>>,
  ) -> AttributesMeshEntities {
    let r = self.write_attribute_mesh_data_uri(mesh, buffer_source);
    self
      .mesh_writer
      .mesh
      .write::<AttributeMeshIsSolid>(r.mesh, true);
    r
  }
}

pub fn use_is_solid_filter(cx: &mut QueryGPUHookCx) -> Option<IsSolidFilter> {
  let att_mesh_to_std_model = cx.use_db_rev_ref_tri_view::<StandardModelRefAttributesMeshEntity>();
  let sm_std_model = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  let att_mesh = cx
    .use_dual_query::<AttributeMeshIsSolid>()
    .fanout(att_mesh_to_std_model, cx)
    .fanout(sm_std_model, cx)
    .dual_query_boxed();

  let is_solid = cx
    .use_dual_query_set::<SceneModelEntity>()
    .dual_query_union(att_mesh, |(scope, v)| {
      match (scope, v) {
        (None, None) => None,
        (None, Some(_)) => unreachable!(),
        (Some(_), None) => Some(false), // default to not solid
        (Some(_), Some(v)) => Some(v),
      }
    })
    .use_dual_query_materialized_hashmap(cx, "scene model is solid");

  let (is_solid, is_solid_) = is_solid.fork();

  let is_solid_ = is_solid_.use_assure_result(cx);

  let (cx, storages) = cx.use_storage_buffer::<u32>("scene model is solid(device)", 128, u32::MAX);

  is_solid
    .into_delta_change()
    .map_changes(|v| if v { 1 } else { 0 })
    .update_storage_array(cx, storages, 0);

  storages.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);
  storages.use_update(cx);

  cx.when_render(|| IsSolidFilter {
    is_solid_device: storages.get_gpu_buffer(),
    is_solid_host: is_solid_.expect_resolve_stage().view.into_boxed(),
  })
}

pub struct IsSolidFilter {
  is_solid_device: AbstractReadonlyStorageBuffer<[u32]>,
  is_solid_host: BoxedDynQuery<RawEntityHandle, bool>,
}

impl IsSolidFilter {
  pub fn execute(&self, batch: &mut SceneModelRenderBatch, cx: &mut FrameCtx) {
    match batch {
      SceneModelRenderBatch::Device(batch) => {
        if let Some(batch) = batch {
          let culler = GPUIsSolidFilter {
            is_solid_device: self.is_solid_device.clone(),
          };

          cx.access_parallel_compute(|cx| {
            cx.scope(|cx| {
              *batch = batch.use_culled_list_and_do_culling(cx, Box::new(culler));
            })
          });
        }
      }
      SceneModelRenderBatch::Host(host_render_batch) => {
        *host_render_batch = Box::new(
          HostIsSolidFilter {
            internal: host_render_batch.clone(),
            is_solid_host: self.is_solid_host.clone(),
          }
          .materialize(),
        )
      }
    }
  }
}

#[derive(Clone)]
struct GPUIsSolidFilter {
  is_solid_device: AbstractReadonlyStorageBuffer<[u32]>,
}

impl ShaderHashProvider for GPUIsSolidFilter {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for GPUIsSolidFilter {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(GPUIsSolidFilterInvocation {
      is_solid_device: cx.bind_by(&self.is_solid_device),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.is_solid_device);
  }
}

struct GPUIsSolidFilterInvocation {
  is_solid_device: ShaderReadonlyPtrOf<[u32]>,
}

impl AbstractCullerInvocation for GPUIsSolidFilterInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    self.is_solid_device.index(id).load().not_equals(1)
  }
}

#[derive(Clone)]
struct HostIsSolidFilter {
  internal: Box<dyn HostRenderBatch>,
  is_solid_host: BoxedDynQuery<RawEntityHandle, bool>,
}

impl HostRenderBatch for HostIsSolidFilter {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(
      self
        .internal
        .iter_scene_models()
        .filter(|v| self.is_solid_host.access(v.raw_handle_ref()).unwrap()),
    )
  }
}
