use rendiation_shader_library::shader_uv_space_to_world_space;

use crate::*;

#[derive(Default)]
pub struct DefaultRtxCameraRenderImplProvider {
  uniforms: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn RtxCameraRenderImpl>> for DefaultRtxCameraRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniforms = camera_gpus(cx);
    self.uniforms = source.register_multi_updater(uniforms);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.uniforms);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn RtxCameraRenderImpl> {
    Box::new(DefaultRtxCameraRenderImpl {
      uniforms: res.take_multi_updater_updated(self.uniforms).unwrap(),
    })
  }
}

pub trait RtxCameraRenderImpl {
  fn get_rtx_camera(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Box<dyn RtxCameraRenderComponent>;
}

pub struct DefaultRtxCameraRenderImpl {
  uniforms: LockReadGuardHolder<CameraUniforms>,
}

impl RtxCameraRenderImpl for DefaultRtxCameraRenderImpl {
  fn get_rtx_camera(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Box<dyn RtxCameraRenderComponent> {
    Box::new(DefaultRtxCameraRenderComponent {
      camera: self.uniforms.get(&camera).unwrap().clone(),
    })
  }
}

pub trait RtxCameraRenderComponent: ShaderHashProvider + DynClone {
  fn build_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn RtxCameraRenderInvocation>;
  fn bind(&self, binding: &mut BindingBuilder);
}
clone_trait_object!(RtxCameraRenderComponent);

#[derive(Clone)]
pub struct DefaultRtxCameraRenderComponent {
  camera: UniformBufferDataView<CameraGPUTransform>,
}

impl ShaderHashProvider for DefaultRtxCameraRenderComponent {
  shader_hash_type_id! {}
}

impl RtxCameraRenderComponent for DefaultRtxCameraRenderComponent {
  fn build_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn RtxCameraRenderInvocation> {
    Box::new(DefaultRtxCameraInvocation {
      camera: binding.bind_by(&self.camera),
    })
  }

  fn bind(&self, binding: &mut BindingBuilder) {
    binding.bind(&self.camera);
  }
}

pub trait RtxCameraRenderInvocation: DynClone {
  // uv is ranged from 0. to 1.
  fn generate_ray(&self, uv: Node<Vec2<f32>>) -> ShaderRay;
}

clone_trait_object!(RtxCameraRenderInvocation);

#[derive(Clone)]
pub struct DefaultRtxCameraInvocation {
  camera: UniformNode<CameraGPUTransform>,
}

impl RtxCameraRenderInvocation for DefaultRtxCameraInvocation {
  fn generate_ray(&self, uv: Node<Vec2<f32>>) -> ShaderRay {
    let view_projection_inv =
      CameraGPUTransform::uniform_node_view_projection_inv_field_ptr(self.camera).load();
    let world = CameraGPUTransform::uniform_node_world_field_ptr(self.camera).load();

    let world_target = shader_uv_space_to_world_space(view_projection_inv, uv, val(1.));

    let origin = world.position();

    let direction = (world_target - origin).normalize();

    ShaderRay { origin, direction }
  }
}
