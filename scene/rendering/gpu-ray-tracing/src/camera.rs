use crate::*;

#[derive(Default)]
pub struct DefaultRtxCameraRenderImplProvider {
  uniforms: UpdateResultToken,
}
pub struct DefaultRtxCameraRenderImpl {
  uniforms: LockReadGuardHolder<CameraUniforms>,
}

impl RenderImplProvider<Box<dyn RtxCameraRenderImpl>> for DefaultRtxCameraRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let camera_uniforms = camera_gpus(cx);
    todo!()
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    todo!()
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn RtxCameraRenderImpl> {
    todo!()
  }
}

pub trait RtxCameraRenderImpl {
  fn get_rtx_camera(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Box<dyn RtxCameraRenderComponent>;
}

pub trait RtxCameraRenderComponent: ShaderHashProvider {
  fn build_invocation(&self) -> Box<dyn RtxCameraRenderInvocation>;
  fn bind(&self, binding: &mut BindingBuilder);
}

pub trait RtxCameraRenderInvocation {
  fn generate_ray(&self, normalized_position: Node<Vec2<f32>>) -> ShaderRay;
}
