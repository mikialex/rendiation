use crate::*;

pub trait GLESCameraRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>>;
}

pub struct DefaultGLESCameraRenderImplProvider;
pub struct DefaultGLESCameraRenderImpl {
  uniforms: LockReadGuardHolder<CameraUniforms>,
}

impl RenderImplProvider<Box<dyn GLESCameraRenderImpl>> for DefaultGLESCameraRenderImplProvider {
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    // let projection = global_watch()

    // let uniforms = camera_gpus(res.cx());
    // res.register_multi_updater(uniforms);
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESCameraRenderImpl> {
    Box::new(DefaultGLESCameraRenderImpl {
      uniforms: res.get_multi_updater().unwrap(),
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
