use crate::*;

#[derive(Default)]
pub struct ScenePTLightingSource {
  point_lights: QueryToken,
}

impl ScenePTLightingSource {
  pub fn register_resource(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let data = point_storage(cx);
    self.point_lights = qcx.register_multi_updater(data);
  }

  pub fn deregister_resource(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.point_lights);
  }

  pub fn create_impl(&self, cx: &mut QueryResultCtx) -> ScenePTLightingSceneData {
    ScenePTLightingSceneData {
      point_lights: cx.take_storage_array_buffer(self.point_lights).unwrap(),
    }
  }
}

#[derive(Clone)]
pub struct ScenePTLightingSceneData {
  point_lights: StorageBufferReadonlyDataView<[PointLightStorage]>,
}

#[derive(Clone)]
pub struct ScenePTLighting {
  pub scene_id: UniformBufferDataView<Vec4<u32>>,
  pub scene_data: ScenePTLightingSceneData,
}

impl ShaderHashProvider for ScenePTLighting {
  shader_hash_type_id! {}
}

impl DevicePathTracingLighting for ScenePTLighting {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingLightingInvocation> {
    let points = cx.bind_by(&self.scene_data.point_lights);
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
    cx.bind(&self.scene_data.point_lights);
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
    self
      .point_lights
      .importance_sampling_light(world_position, sampler)
  }
}
