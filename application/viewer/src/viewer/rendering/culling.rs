use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub fn use_camera_gpu_frustum(qcx: &mut impl QueryGPUHookCx) -> Option<CameraGPUFrustums> {
  qcx
    .use_uniform_buffers(|source, cx| {
      let c = camera_source
        .collective_map(|transform| {
          let arr = Frustum::new_from_matrix(transform.view_projection)
            .planes
            .map(|p| Vec4::new(p.normal.x, p.normal.y, p.normal.z, p.constant));

          Shader140Array::<Vec4<f32>, 6>::from_slice_clamp_or_default(&arr);
        })
        .into_query_update_uniform(0, cx);

      source.with_source(c)
    })
    .map(|frustums| CameraGPUFrustums { frustums })
}

type CameraGPUFrustumsUniform =
  UniformUpdateContainer<EntityHandle<SceneCameraEntity>, Shader140Array<Vec4<f32>, 6>>;

pub struct CameraGPUFrustums {
  frustums: LockReadGuardHolder<CameraGPUFrustumsUniform>,
}

impl CameraGPUFrustums {
  pub fn get_gpu_frustum(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> UniformBufferDataView<Shader140Array<Vec4<f32>, 6>> {
    self.frustums.get(&camera).unwrap().clone()
  }
}
