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

// pub struct DirectionalStorageLightList {
//   token: UpdateResultToken,
// }

// impl RenderImplProvider<Box<dyn LightingComputeComponent>> for DirectionalStorageLightList {
//   fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
//     let uniform = directional_uniform_array(cx);
//     self.token = source.register_multi_updater(uniform);
//   }

//   fn create_impl(
//     &self,
//     res: &mut ConcurrentStreamUpdateResult,
//   ) -> Box<dyn LightingComputeComponent> {
//     let uniform = res
//       .take_multi_updater_updated::<UniformArray<DirectionalLightUniform, 8>>(self.token)
//       .unwrap();
//     let com = ArrayLights(
//       MultiUpdateContainerImplAbstractBindingSource(uniform),
//       |(_, light_uniform): (Node<u32>, UniformNode<DirectionalLightUniform>)| {
//         let light_uniform = light_uniform.load().expand();
//         ENode::<DirectionalShaderInfo> {
//           illuminance: light_uniform.illuminance,
//           direction: light_uniform.direction,
//         }
//         .construct()
//       },
//     );
//     Box::new(com)
//   }
// }
