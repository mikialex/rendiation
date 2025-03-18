use crate::*;

#[derive(Default)]
pub struct ScenePTLightingSource {
  point_lights: QueryToken,
  point_lights_multi_access: QueryToken,
}

impl ScenePTLightingSource {
  pub fn register_resource(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let data = point_storage(cx);
    self.point_lights = qcx.register_multi_updater(data);

    let multi_access = MultiAccessGPUDataBuilder::new(
      cx,
      global_rev_ref().watch_inv_ref_untyped::<PointLightRefScene>(),
      light_multi_access_config(),
    );
    self.point_lights_multi_access = qcx.register(Box::new(multi_access));
  }

  pub fn deregister_resource(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.point_lights);
    qcx.deregister(&mut self.point_lights_multi_access);
  }

  pub fn create_impl(&self, cx: &mut QueryResultCtx) -> ScenePTLightingSceneData {
    ScenePTLightingSceneData {
      point_lights: cx.take_storage_array_buffer(self.point_lights).unwrap(),
      point_light_accessor: cx
        .take_multi_access_gpu(self.point_lights_multi_access)
        .unwrap(),
    }
  }
}

#[derive(Clone)]
pub struct ScenePTLightingSceneData {
  point_lights: StorageBufferReadonlyDataView<[PointLightStorage]>,
  point_light_accessor: MultiAccessGPUData,
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
    let accessor = self.scene_data.point_light_accessor.build(cx);
    let scene_id = cx.bind_by(&self.scene_id).load().x();

    let light_count = accessor.meta.index(scene_id).len().load();

    Box::new(ScenePTLightingInvocation {
      point_lights: LightingGroup {
        strategy: Arc::new(UniformLightSamplingStrategy { light_count }),
        lights: points,
        light_access: accessor,
        scene_id,
      },
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.scene_data.point_lights);
    self.scene_data.point_light_accessor.bind(cx);
    cx.bind(&self.scene_id);
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
