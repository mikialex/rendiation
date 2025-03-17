use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightStorage {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn directional_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<DirectionalLightStorage> {
  let illuminance_offset = offset_of!(DirectionalLightStorage, illuminance);
  let illuminance = global_watch()
    .watch::<DirectionalLightIlluminance>()
    .into_query_update_storage(illuminance_offset);

  let direction = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
    .collective_map(|mat| mat.forward().reverse().normalize())
    .into_query_update_storage(offset_of!(DirectionalLightStorage, direction));

  create_reactive_storage_buffer_container(128, u32::MAX, gpu)
    .with_source(illuminance)
    .with_source(direction)
}

#[derive(Default)]
pub struct DirectionalStorageLightList {
  token: QueryToken,
}

impl QueryBasedFeature<Box<dyn LightingComputeComponent>> for DirectionalStorageLightList {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let data = directional_storage(cx);
    self.token = qcx.register_multi_updater(data);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.token);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightingComputeComponent> {
    let buffer = cx
      .take_storage_array_buffer::<DirectionalLightStorage>(self.token)
      .unwrap();

    let com = ArrayLights(
      buffer,
      |(_, light): (Node<u32>, ShaderReadonlyPtrOf<DirectionalLightStorage>)| {
        let light = light.load().expand();
        ENode::<DirectionalShaderInfo> {
          illuminance: light.illuminance,
          direction: light.direction,
        }
        .construct()
      },
    );
    Box::new(com)
  }
}
