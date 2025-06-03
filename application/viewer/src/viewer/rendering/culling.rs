use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub fn use_camera_gpu_frustum_uniform(
  cx: &mut QueryGPUHookCx,
  camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
) -> Option<CameraGPUFrustums> {
  cx.use_uniform_buffers(|source| {
    let data = camera_source
      .collective_map(|transform| {
        let arr = Frustum::new_from_matrix(transform.view_projection)
          .planes
          .map(|p| Vec4::new(p.normal.x, p.normal.y, p.normal.z, p.constant));

        Shader140Array::<Vec4<f32>, 6>::from_slice_clamp_or_default(&arr);
      })
      .into_query_update_uniform(0, cx.gpu);

    source.with_source(data)
  })
}

type CameraGPUFrustumsUniform =
  UniformUpdateContainer<EntityHandle<SceneCameraEntity>, Shader140Array<Vec4<f32>, 6>>;
pub type CameraGPUFrustums = LockReadGuardHolder<CameraGPUFrustumsUniform>;
