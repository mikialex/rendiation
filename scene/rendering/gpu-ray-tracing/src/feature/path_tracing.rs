use crate::*;

pub struct DevicePathTracingRenderer {
  // create and maintain rtx pipeline
}

impl DevicePathTracingRenderer {
  pub fn render(&self, input: GPU2DTextureView) -> GPU2DTextureView {
    todo!()
  }
}

pub trait DevicePathTracingSceneSource: ShaderHashProvider + ShaderPassBuilder {
  fn sbt(&self) -> Box<dyn Any>;

  fn pbr_material_info_invocation_accessor(
    &self,
  ) -> Box<dyn DevicePathTracingPhysicalMaterialInvocationAccessor>;

  fn lit_material_info_invocation_accessor(
    &self,
  ) -> Box<dyn DevicePathTracingLitMaterialInvocationAccessor>;

  fn light_info(&self) -> Box<dyn DevicePathTracingInvocationLightingInfo>;
}

pub trait DevicePathTracingPhysicalMaterialInvocationAccessor {
  fn pbr_material_info_access(&self, scene_model_id: Node<u32>) -> Node<Vec3<f32>>;
}

pub trait DevicePathTracingLitMaterialInvocationAccessor {
  fn lit_material_info_access(&self, scene_model_id: Node<u32>) -> Node<Vec3<f32>>;
}

pub trait DevicePathTracingInvocationLightingInfo {
  // some light access methods
}
