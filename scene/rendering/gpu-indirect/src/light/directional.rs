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
  let illuminance = global_watch().watch::<DirectionalLightIlluminance>();
  let illuminance_offset = offset_of!(DirectionalLightStorage, illuminance);

  let direction = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
    .collective_map(|mat| mat.forward().reverse().normalize());

  ReactiveStorageBufferContainer::new(gpu)
    .with_source(illuminance, illuminance_offset)
    .with_source(direction, offset_of!(DirectionalLightStorage, direction))
}

pub struct DirectionalStorageLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightingComputeComponent>> for DirectionalStorageLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let data = directional_storage(cx);
    self.token = source.register_multi_updater(data.inner);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.token);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn LightingComputeComponent> {
    let buffer = res
      .take_multi_updater_updated::<CommonStorageBufferImpl<DirectionalLightStorage>>(self.token)
      .unwrap()
      .gpu()
      .clone();

    let com = ArrayLights(
      buffer,
      |(_, light): (Node<u32>, ReadonlyStorageNode<DirectionalLightStorage>)| {
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
