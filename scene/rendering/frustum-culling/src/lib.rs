use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_shader_library::plane::*;
use rendiation_webgpu::*;

type GPUFrustumData = UniformBufferDataView<Shader140Array<ShaderPlaneUniform, 6>>;

#[derive(Clone)]
pub struct GPUFrustumCuller {
  pub bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
  pub frustum: GPUFrustumData,
  pub camera: CameraGPU,
}

impl ShaderHashProvider for GPUFrustumCuller {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for GPUFrustumCuller {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    let ubo = cx.bind_by(&self.camera.ubo);
    let camera_world = hpt_uniform_to_hpt(ubo.world_position().load());

    let frustum = cx.bind_by(&self.frustum);

    let frustum = std::array::from_fn(|i| {
      let plane = frustum.index(val(i as u32)).load();
      ShaderPlaneUniform::into_shader_plane(plane, camera_world)
    });

    Box::new(GPUFrustumCullingInvocation {
      bounding_provider: self.bounding_provider.create_invocation(cx),
      frustum,
      camera_world,
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.camera.ubo);
    cx.bind(&self.frustum);
    self.bounding_provider.bind(cx);
  }
}

struct GPUFrustumCullingInvocation {
  bounding_provider: Box<dyn DrawUnitWorldBoundingInvocationProvider>,
  frustum: [ENode<ShaderPlane>; 6],
  camera_world: Node<HighPrecisionTranslation>,
}

impl AbstractCullerInvocation for GPUFrustumCullingInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    let bounding = self.bounding_provider.get_world_bounding(id);

    let visible = val(false).make_local_var();

    for plane in self.frustum.iter() {
      // todo use a real loop to avoid per plane visible check
      if_by(visible.load().not(), || {
        let min = hpt_sub_hpt(bounding.min, self.camera_world);
        let max = hpt_sub_hpt(bounding.max, self.camera_world);
        let intersect = aabb_plane_intersect(min, max, *plane);
        if_by(intersect, || {
          visible.store(true);
        });
      });
    }

    visible.load()
  }
}
