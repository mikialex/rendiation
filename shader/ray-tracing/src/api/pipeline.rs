use crate::*;

#[derive(Clone)]
pub struct GPURaytracingPipelineDescriptor {
  pub max_recursion_depth: u32,
  pub ray_gen_shaders: Vec<(Box<dyn TraceOperator<()>>, ShaderSizedValueType)>,
  pub miss_hit_shaders: Vec<(Box<dyn TraceOperator<()>>, ShaderSizedValueType)>,
  pub closest_hit_shaders: Vec<(Box<dyn TraceOperator<()>>, ShaderSizedValueType)>,
  pub intersection_shaders: Vec<Arc<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  pub any_hit_shaders: Vec<Arc<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
}

impl GPURaytracingPipelineDescriptor {
  pub fn compute_hash(&self) -> u64 {
    let mut hasher = PipelineHasher::default();
    // note, the payload should have already been hashed in trace operator
    for (s, _) in &self.ray_gen_shaders {
      s.hash_pipeline_with_type_info(&mut hasher);
    }
    for (s, _) in &self.miss_hit_shaders {
      s.hash_pipeline_with_type_info(&mut hasher);
    }
    for (s, _) in &self.closest_hit_shaders {
      s.hash_pipeline_with_type_info(&mut hasher);
    }
    hasher.finish()
  }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u32, pub RayTracingShaderStage);

impl GPURaytracingPipelineDescriptor {
  pub fn set_max_recursion_depth(&mut self, max_recursion_depth: u32) -> &mut Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn register_ray_gen<P: ShaderSizedValueNodeType>(
    &mut self,
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.ray_gen_shaders.len() as u32;
    self
      .ray_gen_shaders
      .push((Box::new(ray_logic), P::sized_ty()));
    ShaderHandle(idx, RayTracingShaderStage::RayGeneration)
  }
  pub fn register_ray_miss<P: ShaderSizedValueNodeType>(
    &mut self,
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.miss_hit_shaders.len() as u32;
    self
      .miss_hit_shaders
      .push((Box::new(ray_logic), P::sized_ty()));
    ShaderHandle(idx, RayTracingShaderStage::Miss)
  }

  pub fn register_ray_closest_hit<P: ShaderSizedValueNodeType>(
    &mut self,
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.closest_hit_shaders.len() as u32;
    self
      .closest_hit_shaders
      .push((Box::new(ray_logic), P::sized_ty()));
    ShaderHandle(idx, RayTracingShaderStage::ClosestHit)
  }

  pub fn register_ray_intersection(
    &mut self,
    builder: impl Fn(&RayIntersectCtx, &dyn IntersectionReporter) + 'static,
  ) -> ShaderHandle {
    let idx = self.intersection_shaders.len() as u32;
    self.intersection_shaders.push(Arc::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::Intersection)
  }

  pub fn register_ray_any_hit(
    &mut self,
    builder: impl Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior> + 'static,
  ) -> ShaderHandle {
    let idx = self.any_hit_shaders.len() as u32;
    self.any_hit_shaders.push(Arc::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::AnyHit)
  }
}
