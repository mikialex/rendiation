use crate::*;

pub struct RayGenShaderCtx {
  launch_id: Node<Vec3<u32>>,
  launch_size: Node<Vec3<u32>>,
}

impl RayDispatchShaderStageCtx for RayGenShaderCtx {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    self.launch_id
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    self.launch_size
  }
}

pub struct RayClosestHitCtx {
  //
}

impl RayClosestHitCtx {}
