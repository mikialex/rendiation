use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightStorage {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

pub fn spot_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<SpotLightStorage> {
  let luminance_intensity = global_watch()
    .watch::<SplitLightIntensity>()
    .into_query_update_storage(offset_of!(SpotLightStorage, luminance_intensity));

  let cutoff_distance = global_watch()
    .watch::<SpotLightCutOffDistance>()
    .into_query_update_storage(offset_of!(SpotLightStorage, cutoff_distance));

  let half_cone_cos = global_watch()
    .watch::<SpotLightHalfConeAngle>()
    .collective_map(|rad| rad.cos())
    .into_query_update_storage(offset_of!(SpotLightStorage, half_cone_cos));

  let half_penumbra_cos = global_watch()
    .watch::<SpotLightHalfPenumbraAngle>()
    .collective_map(|rad| rad.cos())
    .into_query_update_storage(offset_of!(SpotLightStorage, half_penumbra_cos));

  let world = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
    .into_forker();

  let position = world
    .clone()
    .collective_map(|mat| mat.position())
    .into_query_update_storage(offset_of!(SpotLightStorage, position));

  let direction = world
    .collective_map(|mat| mat.forward().reverse().normalize())
    .into_query_update_storage(offset_of!(SpotLightStorage, direction));

  create_reactive_storage_buffer_container(128, u32::MAX, gpu)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(half_cone_cos)
    .with_source(half_penumbra_cos)
    .with_source(position)
    .with_source(direction)
}

#[derive(Default)]
pub struct SpotLightStorageLightList {
  token: QueryToken,
}

impl QueryBasedFeature<Box<dyn LightingComputeComponent>> for SpotLightStorageLightList {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let buffer = spot_storage(cx);
    self.token = qcx.register_multi_updater(buffer);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.token);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightingComputeComponent> {
    let buffer = cx
      .take_storage_array_buffer::<SpotLightStorage>(self.token)
      .unwrap();

    let com = ArrayLights(
      buffer,
      |(_, light_buffer): (Node<u32>, ShaderReadonlyPtrOf<SpotLightStorage>)| {
        let light_buffer = light_buffer.load().expand();
        ENode::<SpotLightShaderInfo> {
          luminance_intensity: light_buffer.luminance_intensity,
          position: light_buffer.position,
          direction: light_buffer.direction,
          cutoff_distance: light_buffer.cutoff_distance,
          half_cone_cos: light_buffer.half_cone_cos,
          half_penumbra_cos: light_buffer.half_penumbra_cos,
        }
        .construct()
      },
    );
    Box::new(com)
  }
}
