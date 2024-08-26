use crate::*;

pub struct GPURaytracingPipelineDescriptor {
  pub max_recursion_depth: u32,
  pub ray_gen_shaders: Vec<Box<dyn TraceOperator<()>>>,
  pub miss_hit_shaders: Vec<Box<dyn FnOnce(&mut RayMissCtx)>>,
  pub closest_hit_shaders: Vec<Box<dyn TraceOperator<()>>>,
  pub intersection_shaders: Vec<Box<dyn FnOnce(&mut RayIntersectCtx)>>,
  pub any_hit_shaders: Vec<Box<dyn FnOnce(&mut RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
}

impl Default for GPURaytracingPipelineDescriptor {
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

pub struct ShaderHandle(pub usize, pub RayTracingShaderStage);

impl GPURaytracingPipelineDescriptor {
  pub fn set_max_recursion_depth(&mut self, max_recursion_depth: u32) -> &mut Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn register_ray_gen(mut self, ray_logic: impl TraceOperator<()> + 'static) -> ShaderHandle {
    let idx = self.ray_gen_shaders.len();
    self.ray_gen_shaders.push(Box::new(ray_logic));
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
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.closest_hit_shaders.len();
    self.closest_hit_shaders.push(Box::new(ray_logic));
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
