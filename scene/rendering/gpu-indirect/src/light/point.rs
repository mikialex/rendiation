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

  create_reactive_storage_buffer_container(gpu)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(position)
}

#[derive(Default)]
pub struct PointStorageLightList {
  token: QueryToken,
}

impl QueryBasedFeature<Box<dyn LightingComputeComponent>> for PointStorageLightList {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let data = point_storage(cx);
    self.token = qcx.register_multi_updater(data);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.token);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightingComputeComponent> {
    let buffer = cx
      .take_multi_updater_updated::<CommonStorageBufferImpl<PointLightStorage>>(self.token)
      .unwrap()
      .gpu()
      .clone();

    let com = ArrayLights(
      buffer,
      |(_, light): (Node<u32>, ShaderReadonlyPtrOf<PointLightStorage>)| {
        let light = light.load().expand();
        ENode::<PointLightShaderInfo> {
          luminance_intensity: light.luminance_intensity,
          position: light.position,
          cutoff_distance: light.cutoff_distance,
        }
        .construct()
      },
    );
    Box::new(com)
  }
}
