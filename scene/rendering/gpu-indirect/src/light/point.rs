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

fn point_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<PointLightStorage> {
  let luminance_intensity = global_watch().watch::<PointLightIntensity>();
  let luminance_intensity_offset = offset_of!(PointLightStorage, luminance_intensity);

  let cutoff_distance = global_watch().watch::<PointLightCutOffDistance>();
  let cutoff_distance_offset = offset_of!(PointLightStorage, cutoff_distance);

  let position = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<PointLightRefNode>())
    .collective_map(|mat| mat.position());

  ReactiveStorageBufferContainer::new(gpu)
    .with_source(luminance_intensity, luminance_intensity_offset)
    .with_source(cutoff_distance, cutoff_distance_offset)
    .with_source(position, offset_of!(PointLightStorage, position))
}

pub struct PointStorageLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightingComputeComponent>> for PointStorageLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let data = point_storage(cx);
    self.token = source.register_multi_updater(data.inner);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.token);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn LightingComputeComponent> {
    let buffer = res
      .take_multi_updater_updated::<CommonStorageBufferImpl<PointLightStorage>>(self.token)
      .unwrap()
      .gpu()
      .clone();

    let com = ArrayLights(
      buffer,
      |(_, light): (Node<u32>, ReadOnlyStorageNode<PointLightStorage>)| {
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
