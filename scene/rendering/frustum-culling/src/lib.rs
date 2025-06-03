use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_shader_library::plane::*;
use rendiation_webgpu::*;

type GPUFrustumData = UniformBufferDataView<Shader140Array<ShaderPlane, 6>>;

#[derive(Clone)]
pub struct GPUFrustumCuller {
  pub bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
  pub frustum: GPUFrustumData,
}

impl ShaderHashProvider for GPUFrustumCuller {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for GPUFrustumCuller {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(GPUFrustumCullingInvocation {
      bounding_provider: self.bounding_provider.create_invocation(cx),
      frustum: cx.bind_by(&self.frustum),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    self.bounding_provider.bind(cx);
    cx.bind(&self.frustum);
  }
}

struct GPUFrustumCullingInvocation {
  bounding_provider: Box<dyn DrawUnitWorldBoundingInvocationProvider>,
  frustum: ShaderReadonlyPtrOf<Shader140Array<ShaderPlane, 6>>,
}

impl AbstractCullerInvocation for GPUFrustumCullingInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    let bounding = self.bounding_provider.get_world_bounding(id);

    let visible = val(false).make_local_var();
    self
      .frustum
      .clone()
      .into_shader_iter()
      .for_each(|(_, plane), cx| {
        let plane = plane.load().expand();
        let intersect = aabb_plane_intersect(bounding.min, bounding.max, plane);
        if_by(intersect, || {
          visible.store(true);
          cx.do_break();
        });
      });

    visible.load()
  }
}
