use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub struct CameraGPUFrustumsSource {
  frustums: QueryToken,
}

impl CameraGPUFrustumsSource {
  pub fn register(
    &mut self,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
    qcx: &mut ReactiveQueryCtx,
    cx: &GPU,
  ) {
    let source = camera_source
      .collective_map(|transform| {
        let arr = Frustum::new_from_matrix(transform.view_projection)
          .planes
          .map(|p| Vec4::new(p.normal.x, p.normal.y, p.normal.z, p.constant));

        Shader140Array::<Vec4<f32>, 6>::from_slice_clamp_or_default(&arr);
      })
      .into_query_update_uniform(0, cx);

    let source = CameraGPUFrustumsUniform::default().with_source(source);

    qcx.register_multi_updater(source);
  }

  pub fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.frustums);
  }

  pub fn create_impl(&self, cx: &mut QueryResultCtx) -> CameraGPUFrustums {
    CameraGPUFrustums {
      frustums: cx.take_multi_updater_updated(self.frustums).unwrap(),
    }
  }
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
