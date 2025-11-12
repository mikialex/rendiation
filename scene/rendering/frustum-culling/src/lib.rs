use std::hash::Hash;

use database::*;
use rendiation_geometry::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_shader_library::plane::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

type GPUFrustumDataType = Shader140Array<Vec4<f32>, 6>;
type GPUFrustumData = UniformBufferDataView<GPUFrustumDataType>;

pub fn use_camera_gpu_frustum(
  cx: &mut QueryGPUHookCx,
  ndc: impl NDCSpaceMapper + Copy + Hash,
) -> Option<CameraGPUFrustums> {
  let uniforms = cx.use_uniform_buffers();

  let host_camera_frustums = cx
    .use_shared_dual_query(GlobalCameraTransformShare(ndc))
    .dual_query_map(move |transform| {
      let mat = ndc.transform_into_opengl_standard_ndc().into_f64() * transform.view_projection;
      Frustum::new_from_matrix(mat)
    });

  let device_camera_frustums = cx
    .use_shared_dual_query(GlobalCameraTransformShare(ndc))
    .dual_query_map(move |transform| {
      let mat = ndc.transform_into_opengl_standard_ndc().into_f64()
        * transform.projection.into_f64()
        * transform.view.remove_position();
      Frustum::new_from_matrix(mat)
    });

  device_camera_frustums
    .use_assure_result(cx)
    .into_delta_change()
    .map_changes(|v| {
      let arr = v
        .planes
        .map(|p| Vec4::new(p.normal.x, p.normal.y, p.normal.z, p.constant).into_f32());
      GPUFrustumDataType::from_slice_clamp_or_default(&arr);
    })
    .update_uniforms(&uniforms, 0, cx.gpu);

  let host_camera_frustums = host_camera_frustums.map(|v| v.view()).use_assure_result(cx);

  cx.when_render(|| CameraGPUFrustums {
    device: uniforms.make_read_holder(),
    host: host_camera_frustums.expect_resolve_stage().into_boxed(),
  })
}

type CameraGPUFrustumsUniform = UniformBufferCollectionRaw<RawEntityHandle, GPUFrustumDataType>;

pub struct CameraGPUFrustums {
  device: LockReadGuardHolder<CameraGPUFrustumsUniform>,
  host: BoxedDynQuery<RawEntityHandle, Frustum<f64>>,
}

impl CameraGPUFrustums {
  pub fn get_gpu_frustum(&self, camera: EntityHandle<SceneCameraEntity>) -> GPUFrustumData {
    self.device.get(&camera.into_raw()).unwrap().clone()
  }
  pub fn get_frustum(&self, camera: EntityHandle<SceneCameraEntity>) -> Frustum<f64> {
    self.host.access(&camera.into_raw()).unwrap()
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

    let frustum = std::array::from_fn(|i| frustum.index(val(i as u32)).load());

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
  frustum: [Node<Vec4<f32>>; 6],
  camera_world: Node<HighPrecisionTranslation>,
}

impl AbstractCullerInvocation for GPUFrustumCullingInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    let bounding = self.bounding_provider.get_world_bounding(id);
    let min = hpt_sub_hpt(bounding.min, self.camera_world);
    let max = hpt_sub_hpt(bounding.max, self.camera_world);

    let should_cull = val(false).make_local_var();

    for plane in self.frustum.iter() {
      let plane = ENode::<ShaderPlane> {
        normal: plane.xyz(),
        constant: plane.w(),
      };

      if_by(should_cull.load().not(), || {
        let intersect = aabb_half_space_intersect(min, max, plane);
        if_by(intersect.not(), || {
          should_cull.store(true);
        });
      });
    }

    should_cull.load()
  }
}

#[derive(Clone)]
pub struct HostFrustumCulling {
  pub inner: Box<dyn HostRenderBatch>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub frustum: Frustum<f64>,
}

impl HostRenderBatch for HostFrustumCulling {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(self.inner.iter_scene_models().filter(|v| {
      let bbox = self.sm_world_bounding.access(v).unwrap();
      self.frustum.intersect(&bbox, &())
    }))
  }
}
