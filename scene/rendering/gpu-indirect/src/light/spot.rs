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
  let luminance_intensity = global_watch().watch::<SplitLightIntensity>();
  let luminance_intensity_offset = offset_of!(SpotLightStorage, luminance_intensity);

  let cutoff_distance = global_watch().watch::<SpotLightCutOffDistance>();
  let cutoff_distance_offset = offset_of!(SpotLightStorage, cutoff_distance);

  let half_cone_cos = global_watch()
    .watch::<SpotLightHalfConeAngle>()
    .collective_map(|rad| rad.cos());
  let half_cone_cos_offset = offset_of!(SpotLightStorage, half_cone_cos);

  let half_penumbra_cos = global_watch()
    .watch::<SpotLightHalfPenumbraAngle>()
    .collective_map(|rad| rad.cos());
  let half_penumbra_cos_offset = offset_of!(SpotLightStorage, half_penumbra_cos);

  let world = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
    .into_forker();

  let position = world.clone().collective_map(|mat| mat.position());
  let position_offset = offset_of!(SpotLightStorage, position);

  let direction = world.collective_map(|mat| mat.forward().reverse().normalize());
  let direction_offset = offset_of!(SpotLightStorage, direction);

  ReactiveStorageBufferContainer::new(gpu)
    .with_source(luminance_intensity, luminance_intensity_offset)
    .with_source(cutoff_distance, cutoff_distance_offset)
    .with_source(half_cone_cos, half_cone_cos_offset)
    .with_source(half_penumbra_cos, half_penumbra_cos_offset)
    .with_source(position, position_offset)
    .with_source(direction, direction_offset)
}

pub struct SpotLightStorageLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightingComputeComponent>> for SpotLightStorageLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let buffer = spot_storage(cx);
    self.token = source.register_multi_updater(buffer.inner);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.token);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn LightingComputeComponent> {
    let buffer = res
      .take_multi_updater_updated::<CommonStorageBufferImpl<SpotLightStorage>>(self.token)
      .unwrap()
      .gpu()
      .clone();

    let com = ArrayLights(
      buffer,
      |(_, light_buffer): (Node<u32>, ReadOnlyStorageNode<SpotLightStorage>)| {
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
