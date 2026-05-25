use rendiation_lighting_punctual::PointLightShaderInfo;
use rendiation_lighting_punctual::PunctualShaderLight;

use crate::*;

pub fn use_scene_point_light_uniform(
  cx: &mut QueryGPUHookCx,
) -> Option<ScenePointLightingProvider> {
  let uniform = use_point_per_scene_uniform_array_buffers(cx);
  cx.when_render(|| ScenePointLightingProvider {
    uniform: uniform.unwrap(),
  })
}

pub struct ScenePointLightingProvider {
  uniform: PointLightUniforms,
}

impl LightSystemSceneProvider for ScenePointLightingProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let lights = self.uniform.1.read().get(scene.raw_handle_ref())?.clone();
    Some(Box::new(PointLightShader { lights }))
  }
}

#[derive(Clone)]
struct PointLightShader {
  lights: UniformBufferDataView<UniformArrayWithLengthInfo<PointLightUniform>>,
}

impl ShaderHashProvider for PointLightShader {
  shader_hash_type_id! {}
}

impl LightingComputeComponent for PointLightShader {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    _scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(PointLightInvocation {
      lights: binding.bind_by(&self.lights),
    })
  }

  fn setup_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.lights);
  }
}

struct PointLightInvocation {
  lights: ShaderReadonlyPtrOf<UniformArrayWithLengthInfo<PointLightUniform>>,
}

impl LightingComputeInvocation for PointLightInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    light_iter_sum(self.lights.clone().into_shader_iter().map(
      |(_, light_ptr): (Node<u32>, ShaderReadonlyPtrOf<PointLightUniform>)| {
        let uniform = light_ptr.load().expand();
        let light = ENode::<PointLightShaderInfo> {
          luminance_intensity: uniform.luminance_intensity,
          position: hpt_uniform_to_hpt(uniform.position),
          cutoff_distance: uniform.cutoff_distance,
        }
        .construct();
        let incident = light.compute_incident_light(geom_ctx);
        shading.compute_lighting_by_incident(
          &ENode::<ShaderIncidentLight> {
            color: incident.color,
            direction: incident.direction,
          },
          geom_ctx,
        )
      },
    ))
  }
}
