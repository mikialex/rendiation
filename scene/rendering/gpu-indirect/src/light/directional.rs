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
  light_data: QueryToken,
  multi_access: QueryToken,
}

pub(crate) fn light_multi_access_config() -> MultiAccessGPUDataBuilderInit {
  MultiAccessGPUDataBuilderInit {
    max_possible_many_count: u32::MAX,
    max_possible_one_count: u32::MAX,
    init_many_count_capacity: 128,
    init_one_count_capacity: 128,
  }
}

impl QueryBasedFeature<Box<dyn LightingComputeComponent>> for DirectionalStorageLightList {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let data = directional_storage(cx);
    self.light_data = qcx.register_multi_updater(data);

    let multi_access = MultiAccessGPUDataBuilder::new(
      cx,
      global_rev_ref().watch_inv_ref_untyped::<DirectionalRefScene>(),
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
      .take_storage_array_buffer::<DirectionalLightStorage>(self.light_data)
      .unwrap();

    let light_accessor = *cx
      .take_result(self.multi_access)
      .unwrap()
      .downcast::<MultiAccessGPUData>()
      .unwrap();

    let lighting = AllInstanceOfGivenTypeLightInScene {
      light_accessor,
      light_data,
      create_per_light_compute: std::sync::Arc::new(
        |light: ShaderReadonlyPtrOf<DirectionalLightStorage>| {
          let light = light.load().expand();
          Box::new(
            ENode::<DirectionalShaderInfo> {
              illuminance: light.illuminance,
              direction: light.direction,
            }
            .construct(),
          )
        },
      ),
    };

    Box::new(lighting)
  }
}
