use crate::*;

#[derive(Default)]
pub struct ScenePTLightingSource {
  point_lights: QueryToken,
  point_lights_multi_access: QueryToken,
  spot_lights: QueryToken,
  spot_lights_multi_access: QueryToken,
  directional_lights: QueryToken,
  directional_lights_multi_access: QueryToken,
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

    let data = spot_storage(cx);
    self.spot_lights = qcx.register_multi_updater(data);

    let multi_access = MultiAccessGPUDataBuilder::new(
      cx,
      global_rev_ref().watch_inv_ref_untyped::<SpotLightRefScene>(),
      light_multi_access_config(),
    );
    self.spot_lights_multi_access = qcx.register(Box::new(multi_access));

    let data = directional_storage(cx);
    self.directional_lights = qcx.register_multi_updater(data);

    let multi_access = MultiAccessGPUDataBuilder::new(
      cx,
      global_rev_ref().watch_inv_ref_untyped::<DirectionalRefScene>(),
      light_multi_access_config(),
    );
    self.directional_lights_multi_access = qcx.register(Box::new(multi_access));
  }

  pub fn deregister_resource(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.point_lights);
    qcx.deregister(&mut self.point_lights_multi_access);
    qcx.deregister(&mut self.spot_lights);
    qcx.deregister(&mut self.spot_lights_multi_access);
    qcx.deregister(&mut self.directional_lights);
    qcx.deregister(&mut self.directional_lights_multi_access);
  }

  pub fn create_impl(&self, cx: &mut QueryResultCtx) -> ScenePTLightingSceneDataGroup {
    ScenePTLightingSceneDataGroup {
      point_lights: ScenePTLightingSceneData {
        lights: cx.take_storage_array_buffer(self.point_lights).unwrap(),
        lights_accessor: cx
          .take_multi_access_gpu(self.point_lights_multi_access)
          .unwrap(),
      },
      spot_lights: ScenePTLightingSceneData {
        lights: cx.take_storage_array_buffer(self.spot_lights).unwrap(),
        lights_accessor: cx
          .take_multi_access_gpu(self.spot_lights_multi_access)
          .unwrap(),
      },
      directional_lights: ScenePTLightingSceneData {
        lights: cx
          .take_storage_array_buffer(self.directional_lights)
          .unwrap(),
        lights_accessor: cx
          .take_multi_access_gpu(self.directional_lights_multi_access)
          .unwrap(),
      },
    }
  }
}

#[derive(Clone)]
pub struct ScenePTLightingSceneDataGroup {
  pub spot_lights: ScenePTLightingSceneData<SpotLightStorage>,
  pub point_lights: ScenePTLightingSceneData<PointLightStorage>,
  pub directional_lights: ScenePTLightingSceneData<DirectionalLightStorage>,
}

#[derive(Clone)]
pub struct ScenePTLightingSceneData<T: Std430> {
  lights: StorageBufferReadonlyDataView<[T]>,
  lights_accessor: MultiAccessGPUData,
}

#[derive(Clone)]
pub struct ScenePTLighting {
  pub scene_id: UniformBufferDataView<Vec4<u32>>,
  pub scene_data: ScenePTLightingSceneDataGroup,
}

impl ShaderHashProvider for ScenePTLighting {
  shader_hash_type_id! {}
}

impl DevicePathTracingLighting for ScenePTLighting {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingLightingInvocation> {
    let scene_id = cx.bind_by(&self.scene_id).load().x();

    // let points = cx.bind_by(&self.scene_data.point_lights.lights);
    // let accessor = self.scene_data.point_lights.lights_accessor.build(cx);
    // let light_count = accessor.meta.index(scene_id).len().load();
    // let points = ScenePTLightingInvocation {
    //   lights: LightingGroup {
    //     strategy: Arc::new(UniformLightSamplingStrategy { light_count }),
    //     lights: points,
    //     light_access: accessor,
    //     scene_id,
    //   },
    // };

    // let spots = cx.bind_by(&self.scene_data.spot_lights.lights);
    // let accessor = self.scene_data.spot_lights.lights_accessor.build(cx);
    // let light_count = accessor.meta.index(scene_id).len().load();
    // let spot = ScenePTLightingInvocation {
    //   lights: LightingGroup {
    //     strategy: Arc::new(UniformLightSamplingStrategy { light_count }),
    //     lights: spots,
    //     light_access: accessor,
    //     scene_id,
    //   },
    // };

    let directional_lights = cx.bind_by(&self.scene_data.directional_lights.lights);
    let accessor = self.scene_data.directional_lights.lights_accessor.build(cx);
    let light_count = accessor.meta.index(scene_id).len().load();
    let directional = ScenePTLightingInvocation {
      lights: LightingGroup {
        strategy: Arc::new(UniformLightSamplingStrategy { light_count }),
        lights: directional_lights,
        light_access: accessor,
        scene_id,
      },
    };

    let group = ScenePTLightingInvocationGroup {
      // points,
      // spot,
      directional,
    };

    Box::new(group)
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.scene_id);

    // cx.bind(&self.scene_data.point_lights.lights);
    // self.scene_data.point_lights.lights_accessor.bind(cx);

    cx.bind(&self.scene_data.directional_lights.lights);
    self.scene_data.directional_lights.lights_accessor.bind(cx);
  }
}

#[derive(Clone)]
struct ScenePTLightingInvocationGroup {
  // points: ScenePTLightingInvocation<PointLightStorage>,
  // spot: ScenePTLightingInvocation<SpotLightStorage>,
  directional: ScenePTLightingInvocation<DirectionalLightStorage>,
}

impl DevicePathTracingLightingInvocation for ScenePTLightingInvocationGroup {
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    self
      .directional
      .importance_sampling_light(world_position, sampler)
  }
}

struct ScenePTLightingInvocation<T: ShaderSizedValueNodeType> {
  lights: LightingGroup<T>,
}

impl<T: ShaderSizedValueNodeType> Clone for ScenePTLightingInvocation<T> {
  fn clone(&self) -> Self {
    Self {
      lights: self.lights.clone(),
    }
  }
}

impl<T> DevicePathTracingLightingInvocation for ScenePTLightingInvocation<T>
where
  T: ShaderSizedValueNodeType + ShaderStructuralNodeType,
  ENode<T>: DevicePathTracingLightingInvocation,
{
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    self
      .lights
      .importance_sampling_light(world_position, sampler)
  }
}
