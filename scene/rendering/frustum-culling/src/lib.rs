use database::*;
use reactive::*;
use rendiation_geometry::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_shader_library::plane::*;
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

type GPUFrustumDataType = Shader140Array<ShaderPlaneUniform, 6>;
type GPUFrustumData = UniformBufferDataView<GPUFrustumDataType>;

pub fn use_camera_gpu_frustum(
  cx: &mut QueryGPUHookCx,
  ndc: impl NDCSpaceMapper + Copy,
) -> Option<CameraGPUFrustums> {
  let uniforms = cx.use_uniform_buffers();

  cx.use_shared_dual_query(GlobalCameraTransformShare(ndc))
    .into_delta_change()
    .map_changes(|transform| {
      let arr = Frustum::new_from_matrix(transform.view_projection)
        .planes
        .map(|p| Vec4::new(p.normal.x, p.normal.y, p.normal.z, p.constant).into_f32());

      Shader140Array::<Vec4<f32>, 6>::from_slice_clamp_or_default(&arr);
    })
    .update_uniforms(&uniforms, 0, cx.gpu);

  cx.when_render(|| CameraGPUFrustums {
    frustums: uniforms.make_read_holder(),
  })
}

type CameraGPUFrustumsUniform = UniformBufferCollectionRaw<RawEntityHandle, GPUFrustumDataType>;

pub struct CameraGPUFrustums {
  frustums: LockReadGuardHolder<CameraGPUFrustumsUniform>,
}

impl CameraGPUFrustums {
  pub fn get_gpu_frustum(&self, camera: EntityHandle<SceneCameraEntity>) -> GPUFrustumData {
    self.frustums.get(&camera.into_raw()).unwrap().clone()
  }
}

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

    let should_cull = val(false).make_local_var();

    for plane in self.frustum.iter() {
      // todo use a real loop to avoid per plane visible check
      if_by(should_cull.load().not(), || {
        let min = hpt_sub_hpt(bounding.min, self.camera_world);
        let max = hpt_sub_hpt(bounding.max, self.camera_world);
        let intersect = aabb_plane_intersect(min, max, *plane);
        if_by(intersect, || {
          should_cull.store(true);
        });
      });
    }

    should_cull.load()
  }
}
