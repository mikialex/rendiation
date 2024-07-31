use crate::*;

pub trait ShaderRayGenLogic:
  DeviceFuture<Ctx = RayGenShaderCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}
impl<T> ShaderRayGenLogic for T where
  T: DeviceFuture<Ctx = RayGenShaderCtx, Output = ()>
    + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}

pub trait ShaderRayGenLogicBoxed:
  DeviceFuture<Ctx = RayGenShaderCtx, Output = (), State = Box<dyn Any>>
  + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}
impl<T> ShaderRayGenLogicBoxed for T where
  T: DeviceFuture<Ctx = RayGenShaderCtx, Output = (), State = Box<dyn Any>>
    + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}

pub trait ShaderRayClosestHitLogic:
  DeviceFuture<Ctx = RayClosestHitCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}
impl<T> ShaderRayClosestHitLogic for T where
  T: DeviceFuture<Ctx = RayClosestHitCtx, Output = ()>
    + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}

pub trait ShaderRayClosestHitLogicBoxed:
  DeviceFuture<Ctx = RayClosestHitCtx, Output = (), State = Box<dyn Any>>
  + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}
impl<T> ShaderRayClosestHitLogicBoxed for T where
  T: DeviceFuture<Ctx = RayClosestHitCtx, Output = (), State = Box<dyn Any>>
    + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}

pub struct GPURaytracingPipelineBuilder {
  pub max_recursion_depth: u32,
  ray_gen_shaders: Vec<Box<dyn ShaderRayGenLogicBoxed>>,
  miss_hit_shaders: Vec<Box<dyn FnOnce(&mut RayMissCtx)>>,
  closest_hit_shaders: Vec<Box<dyn ShaderRayClosestHitLogicBoxed>>,
  intersection_shaders: Vec<Box<dyn FnOnce(&mut RayIntersectCtx)>>,
  any_hit_shaders: Vec<Box<dyn FnOnce(&mut RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
}

impl Default for GPURaytracingPipelineBuilder {
  fn default() -> Self {
    Self {
      max_recursion_depth: 4,
      ray_gen_shaders: Default::default(),
      closest_hit_shaders: Default::default(),
      miss_hit_shaders: Default::default(),
      any_hit_shaders: Default::default(),
      intersection_shaders: Default::default(),
    }
  }
}
pub struct GPURaytracingPipeline {
  pub internal: u32,
}

pub struct GPUShaderBindingTable {
  pub internal: u32,
}

pub enum RayAnyHitBehavior {
  IgnoreThisIntersect,
  TerminateTraverse,
}

pub struct ShaderHandle(pub usize, pub RayTracingShaderStage);

impl GPURaytracingPipelineBuilder {
  pub fn set_max_recursion_depth(&mut self, max_recursion_depth: u32) -> &mut Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn register_ray_gen(mut self, ray_logic: impl ShaderRayGenLogic + 'static) -> ShaderHandle {
    let idx = self.ray_gen_shaders.len();
    self.ray_gen_shaders.push(Box::new(BoxState(ray_logic)));
    ShaderHandle(idx, RayTracingShaderStage::RayGeneration)
  }
  pub fn register_ray_miss(
    mut self,
    builder: impl FnOnce(&mut RayMissCtx) + 'static,
  ) -> ShaderHandle {
    let idx = self.miss_hit_shaders.len();
    self.miss_hit_shaders.push(Box::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::Miss)
  }

  pub fn register_ray_intersection(
    mut self,
    builder: impl FnOnce(&mut RayIntersectCtx) + 'static,
  ) -> ShaderHandle {
    let idx = self.intersection_shaders.len();
    self.intersection_shaders.push(Box::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::Intersection)
  }

  pub fn register_ray_closest_hit(
    &mut self,
    ray_logic: impl ShaderRayClosestHitLogic + 'static,
  ) -> ShaderHandle {
    let idx = self.closest_hit_shaders.len();
    self.closest_hit_shaders.push(Box::new(BoxState(ray_logic)));
    ShaderHandle(idx, RayTracingShaderStage::ClosestHit)
  }

  pub fn register_ray_any_hit(
    &mut self,
    builder: impl FnOnce(&mut RayAnyHitCtx) -> Node<RayAnyHitBehavior> + 'static,
  ) -> ShaderHandle {
    let idx = self.any_hit_shaders.len();
    self.any_hit_shaders.push(Box::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::AnyHit)
  }
}
