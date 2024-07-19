use crate::*;

pub struct GPURaytracingPipelineBuilder {}
pub struct GPURaytracingPipeline {
  pub internal: Box<dyn GPURaytracingPipelineProvider>,
}

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

impl GPURaytracingPipelineBuilder {
  pub fn with_ray_gen<T: ShaderNodeType>(
    self,
    next_trace: impl FnOnce(&mut RayGenShaderCtx) -> (Node<T>, ShaderRayTraceCall, Node<bool>),
    continuation: impl FnOnce(&mut RayGenShaderCtx, Node<T>),
  ) -> Self {
    self
  }
  pub fn with_ray_intersection(self, builder: impl FnOnce(&mut usize)) -> Self {
    self
  }

  pub fn with_ray_closest_hit<T: ShaderNodeType>(
    self,
    range: usize,
    next_trace: impl FnOnce(&mut RayClosestHitCtx) -> (Node<T>, ShaderRayTraceCall, Node<bool>),
    continuation: impl FnOnce(&mut RayClosestHitCtx, Node<T>),
  ) -> Self {
    self
  }

  pub fn with_ray_any_hit(self, range: usize, builder: impl FnOnce(&mut usize)) -> Self {
    self
  }

  pub fn with_ray_miss(self, builder: impl FnOnce(&mut usize)) -> Self {
    self
  }
}

pub struct RayClosestHitCtx {
  //
}

impl RayClosestHitCtx {
  pub fn register_caller_ray_payload_input<T: ShaderNodeType>(self) -> Node<T> {
    todo!()
  }
}
