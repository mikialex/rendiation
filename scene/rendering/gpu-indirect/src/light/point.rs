use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightStorage {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

pub fn point_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<PointLightStorage> {
  let luminance_intensity_offset = offset_of!(PointLightStorage, luminance_intensity);
  let luminance_intensity = global_watch()
    .watch::<PointLightIntensity>()
    .into_query_update_storage(luminance_intensity_offset);

  let cutoff_distance_offset = offset_of!(PointLightStorage, cutoff_distance);
  let cutoff_distance = global_watch()
    .watch::<PointLightCutOffDistance>()
    .into_query_update_storage(cutoff_distance_offset);

  let position = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<PointLightRefNode>())
    .collective_map(|mat| mat.position())
    .into_query_update_storage(offset_of!(PointLightStorage, position));

  create_reactive_storage_buffer_container(128, u32::MAX, gpu)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(position)
}

#[derive(Default)]
pub struct PointStorageLightList {
  light_data: QueryToken,
  multi_access: QueryToken,
}

impl QueryBasedFeature<Box<dyn LightingComputeComponent>> for PointStorageLightList {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let data = point_storage(cx);
    self.light_data = qcx.register_multi_updater(data);

    let multi_access = MultiAccessGPUDataBuilder::new(
      cx,
      global_rev_ref().watch_inv_ref_untyped::<PointLightRefScene>(),
      light_multi_access_config(),
    );
    self.multi_access = qcx.register(Box::new(multi_access));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.light_data);
    qcx.deregister(&mut self.multi_access);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightingComputeComponent> {
    let light_data = cx
      .take_storage_array_buffer::<PointLightStorage>(self.light_data)
      .unwrap();

    let light_accessor = cx.take_multi_access_gpu(self.multi_access).unwrap();

    let lighting = AllInstanceOfGivenTypeLightInScene::new(light_accessor, light_data, |light| {
      let light = light.load().expand();
      ENode::<PointLightShaderInfo> {
        luminance_intensity: light.luminance_intensity,
        position: light.position,
        cutoff_distance: light.cutoff_distance,
      }
      .construct()
    });

    Box::new(lighting)
  }
}
