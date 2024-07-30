use crate::*;

pub trait ShaderRayGenLogic:
  DeviceFuture<Ctx = RayGenShaderCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}

pub trait BoxShaderRayGenLogic {}

pub trait ShaderRayClosestHitLogic:
  DeviceFuture<Ctx = RayClosestHitCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}
pub trait BoxShaderRayClosestHitLogic {}

pub struct GPURaytracingPipelineBuilder {
  pub max_recursion_depth: u32,
  ray_gen_shader: Box<dyn BoxShaderRayGenLogic>,
  miss_hit_shader: Vec<Box<dyn FnOnce(&mut RayMissCtx)>>,
  // intersection_shaders
  closest_hit_shaders: Vec<Box<dyn BoxShaderRayClosestHitLogic>>,
  // any_hit_shaders
}

impl Default for GPURaytracingPipelineBuilder {
  fn default() -> Self {
    Self {
      max_recursion_depth: 8,
      ray_gen_shader: todo!(),
      closest_hit_shaders: Default::default(),
      miss_hit_shader: todo!(),
    }
  }
}
pub struct GPURaytracingPipeline {
  pub internal: Box<dyn GPURaytracingPipelineProvider>,
}

pub enum RayAnyHitBehavior {
  IgnoreThisIntersect,
  TerminateTraverse,
}

pub struct ShaderHandle(usize);

impl GPURaytracingPipelineBuilder {
  pub fn set_max_recursion_depth(&mut self, max_recursion_depth: u32) -> &mut Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn register_ray_gen(mut self, ray_logic: impl ShaderRayGenLogic) -> ShaderHandle {
    todo!()
  }
  pub fn register_ray_miss(
    mut self,
    builder: impl FnOnce(&mut RayMissCtx) + 'static,
  ) -> ShaderHandle {
    let idx = self.miss_hit_shader.len();
    self.miss_hit_shader.push(Box::new(builder));
    ShaderHandle(idx)
  }

  pub fn register_ray_intersection(
    mut self,
    builder: impl FnOnce(&mut RayIntersectCtx),
  ) -> ShaderHandle {
    todo!()
  }

  pub fn register_ray_closest_hit(
    &mut self,
    ray_logic: impl ShaderRayClosestHitLogic,
  ) -> ShaderHandle {
    todo!()
  }

  pub fn register_ray_any_hit(
    &mut self,
    builder: impl FnOnce(&mut RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> ShaderHandle {
    todo!()
  }
}
