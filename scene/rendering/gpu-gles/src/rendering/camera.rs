use crate::*;

pub trait GLESCameraRenderImpl {
  fn make_component(&self, idx: AllocIdx<SceneCameraEntity>)
    -> Option<Box<dyn RenderComponentAny>>;
}

pub struct DefaultGLESCameraRenderImplProvider;
pub struct DefaultGLESCameraRenderImpl {
  uniforms: CameraUniforms,
}

impl RenderImplProvider<Box<dyn GLESCameraRenderImpl>> for DefaultGLESCameraRenderImplProvider {
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    todo!()
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESCameraRenderImpl> {
    Box::new(DefaultGLESCameraRenderImpl { uniforms: todo!() })
  }
}

impl GLESCameraRenderImpl for DefaultGLESCameraRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponentAny>> {
    let node = CameraGPU {
      ubo: self.uniforms.get(&idx)?,
    };

    // Some(Box::new(node))
    todo!()
  }
}
