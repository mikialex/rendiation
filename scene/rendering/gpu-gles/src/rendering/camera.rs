use crate::*;

#[derive(Default)]
pub struct DefaultGLESCameraRenderImplProvider {
  uniforms: UpdateResultToken,
}
pub struct DefaultGLESCameraRenderImpl {
  uniforms: LockReadGuardHolder<CameraUniforms>,
}

impl RenderImplProvider<Box<dyn CameraRenderImpl>> for DefaultGLESCameraRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniforms = camera_gpus(cx);
    self.uniforms = source.register_multi_updater(uniforms);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.uniforms);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn CameraRenderImpl> {
    Box::new(DefaultGLESCameraRenderImpl {
      uniforms: res.take_multi_updater_updated(self.uniforms).unwrap(),
    })
  }
}

impl CameraRenderImpl for DefaultGLESCameraRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = CameraGPU {
      ubo: self.uniforms.get(&idx)?,
    };
    Some(Box::new(node))
  }

  fn make_dep_component(
    &self,
    idx: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderDependencyComponent + '_>> {
    let node = CameraGPU {
      ubo: self.uniforms.get(&idx)?,
    };
    Some(Box::new(node))
  }

  fn setup_camera_jitter(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
    jitter: Vec2<f32>,
    queue: &GPUQueue,
  ) {
    let uniform = self.uniforms.get(&camera).unwrap();
    uniform.write_at(
      queue,
      &jitter,
      offset_of!(CameraGPUTransform, jitter_normalized) as u64,
    );
  }
}
