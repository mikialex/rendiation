use crate::*;

pub trait GLESCameraRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>>;
}

#[derive(Default)]
pub struct DefaultGLESCameraRenderImplProvider {
  uniforms: UpdateResultToken,
}
pub struct DefaultGLESCameraRenderImpl {
  uniforms: LockReadGuardHolder<CameraUniforms>,
}

impl RenderImplProvider<Box<dyn GLESCameraRenderImpl>> for DefaultGLESCameraRenderImplProvider {
  fn register_resource(&mut self, source: &mut ConcurrentStreamContainer, cx: &GPUResourceCtx) {
    let projection = camera_project_matrix();
    let node_mats = scene_node_derive_world_mat();

    let uniforms = camera_gpus(projection, node_mats, cx);
    self.uniforms = source.register_multi_updater(uniforms);
  }

  fn create_impl(&self, res: &ConcurrentStreamUpdateResult) -> Box<dyn GLESCameraRenderImpl> {
    Box::new(DefaultGLESCameraRenderImpl {
      uniforms: res.get_multi_updater(self.uniforms).unwrap(),
    })
  }
}

impl GLESCameraRenderImpl for DefaultGLESCameraRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>> {
    let node = CameraGPU {
      ubo: self.uniforms.get(&idx)?,
    };
    Some(Box::new(node))
  }
}
