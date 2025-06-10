use crate::*;

pub fn use_scene_pt_light_source(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<ScenePTLightingSceneDataGroup> {
  let directional_lights = use_directional_light_storage(qcx);
  let spot_lights = use_spot_light_storage(qcx);
  let point_lights = use_point_light_storage(qcx);

  qcx.when_render(|| ScenePTLightingSceneDataGroup {
    spot_lights: spot_lights.unwrap().into(),
    point_lights: point_lights.unwrap().into(),
    directional_lights: directional_lights.unwrap().into(),
  })
}

impl<T: Std430> From<LightGPUStorage<T>> for ScenePTLightingSceneData<T> {
  fn from(value: LightGPUStorage<T>) -> Self {
    ScenePTLightingSceneData {
      lights: value.0,
      lights_accessor: value.1,
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
