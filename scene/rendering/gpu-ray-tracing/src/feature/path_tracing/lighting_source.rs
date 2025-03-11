use rendiation_webgpu_reactive_utils::CommonStorageBufferImpl;

use crate::*;

#[derive(Default)]
pub struct ScenePTLightingSource {
  point_lights: UpdateResultToken,
}

impl ScenePTLightingSource {
  pub fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let data = point_storage(cx);
    self.point_lights = source.register_multi_updater(data.inner);
  }

  pub fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.point_lights);
  }

  pub fn create_impl(&self, res: &mut QueryResultCtx) -> ScenePTLighting {
    ScenePTLighting {
      point_lights: res
        .take_multi_updater_updated::<CommonStorageBufferImpl<PointLightStorage>>(self.point_lights)
        .unwrap()
        .gpu()
        .clone(),
    }
  }
}

#[derive(Clone)]
pub struct ScenePTLighting {
  point_lights: StorageBufferReadonlyDataView<[PointLightStorage]>,
}

impl ShaderHashProvider for ScenePTLighting {
  shader_hash_type_id! {}
}

impl DevicePathTracingLighting for ScenePTLighting {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingLightingInvocation> {
    let points = cx.bind_by(&self.point_lights);
    Box::new(ScenePTLightingInvocation {
      point_lights: LightingGroup {
        strategy: Arc::new(UniformLightSamplingStrategy {
          light_count: points.array_length(),
        }),
        lights: points,
      },
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.point_lights);
  }
}

#[derive(Clone)]
struct ScenePTLightingInvocation {
  point_lights: LightingGroup<PointLightStorage>,
}

impl DevicePathTracingLightingInvocation for ScenePTLightingInvocation {
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    // self
    //   .point_lights
    //   .importance_sampling_light(world_position, sampler)
    todo!()
  }
}
