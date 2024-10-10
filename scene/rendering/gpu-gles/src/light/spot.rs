use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightUniform {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

pub fn spot_uniform_array(gpu: &GPU) -> UniformArrayUpdateContainer<SpotLightUniform> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let luminance_intensity = global_watch()
    .watch::<SplitLightIntensity>()
    .into_uniform_array_collection_update(offset_of!(SpotLightUniform, luminance_intensity), gpu);

  let cutoff_distance = global_watch()
    .watch::<SpotLightCutOffDistance>()
    .into_uniform_array_collection_update(offset_of!(SpotLightUniform, cutoff_distance), gpu);

  let half_cone_cos = global_watch()
    .watch::<SpotLightHalfConeAngle>()
    .collective_map(|rad| rad.cos())
    .into_uniform_array_collection_update(offset_of!(SpotLightUniform, half_cone_cos), gpu);

  let half_penumbra_cos = global_watch()
    .watch::<SpotLightHalfPenumbraAngle>()
    .collective_map(|rad| rad.cos())
    .into_uniform_array_collection_update(offset_of!(SpotLightUniform, half_penumbra_cos), gpu);

  let world = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
    .into_forker();

  let position = world
    .clone()
    .collective_map(|mat| mat.position())
    .into_uniform_array_collection_update(offset_of!(SpotLightUniform, position), gpu);

  let direction = world
    .collective_map(|mat| mat.forward().reverse().normalize())
    .into_uniform_array_collection_update(offset_of!(SpotLightUniform, direction), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(half_cone_cos)
    .with_source(half_penumbra_cos)
    .with_source(position)
    .with_source(direction)
}

pub struct SpotLightUniformLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightingComputeComponent>> for SpotLightUniformLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniform = directional_uniform_array(cx);
    self.token = source.register_multi_updater(uniform);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn LightingComputeComponent> {
    let uniform = res
      .take_multi_updater_updated::<UniformArray<SpotLightUniform, 8>>(self.token)
      .unwrap()
      .target
      .clone();
    let com = ArrayLights(
      uniform,
      |(_, light_uniform): (Node<u32>, UniformNode<SpotLightUniform>)| {
        let light_uniform = light_uniform.load().expand();
        ENode::<SpotLightShaderInfo> {
          luminance_intensity: light_uniform.luminance_intensity,
          position: light_uniform.position,
          direction: light_uniform.direction,
          cutoff_distance: light_uniform.cutoff_distance,
          half_cone_cos: light_uniform.half_cone_cos,
          half_penumbra_cos: light_uniform.half_penumbra_cos,
        }
        .construct()
      },
    );
    Box::new(com)
  }
}
