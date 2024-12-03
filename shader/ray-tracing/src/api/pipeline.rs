use crate::*;

#[derive(Clone)]
pub struct RayTracingShaderStageDefine {
  pub logic: Box<dyn TraceOperator<()>>,
  pub user_defined_payload_input_ty: ShaderSizedValueType,
}

#[derive(Clone)]
pub struct GPURaytracingPipelineAndBindingSource {
  pub max_recursion_depth: u32,
  pub ray_gen: Vec<RayTracingShaderStageDefine>,
  pub miss_hit: Vec<RayTracingShaderStageDefine>,
  pub closest_hit: Vec<RayTracingShaderStageDefine>,

  // todo, support binding
  pub intersection: Vec<Arc<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  // todo, support binding
  pub any_hit: Vec<Arc<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
}

impl GPURaytracingPipelineAndBindingSource {
  pub fn compute_hash(&self) -> u64 {
    let mut hasher = PipelineHasher::default();
    // note, the payload should have already been hashed in trace operator
    for s in &self.ray_gen {
      s.logic.hash_pipeline_with_type_info(&mut hasher);
    }
    for s in &self.miss_hit {
      s.logic.hash_pipeline_with_type_info(&mut hasher);
    }
    for s in &self.closest_hit {
      s.logic.hash_pipeline_with_type_info(&mut hasher);
    }
    hasher.finish()
  }
}

impl Default for GPURaytracingPipelineAndBindingSource {
  fn default() -> Self {
    Self {
      max_recursion_depth: 1,
      ray_gen: Default::default(),
      closest_hit: Default::default(),
      miss_hit: Default::default(),
      any_hit: Default::default(),
      intersection: Default::default(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u32, pub RayTracingShaderStage);

impl GPURaytracingPipelineAndBindingSource {
  pub fn set_max_recursion_depth(&mut self, max_recursion_depth: u32) -> &mut Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn register_ray_gen<P: ShaderSizedValueNodeType>(
    &mut self,
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.ray_gen.len() as u32;
    let stage = RayTracingShaderStageDefine {
      logic: Box::new(ray_logic),
      user_defined_payload_input_ty: P::sized_ty(),
    };
    self.ray_gen.push(stage);
    ShaderHandle(idx, RayTracingShaderStage::RayGeneration)
  }
  pub fn register_ray_miss<P: ShaderSizedValueNodeType>(
    &mut self,
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.miss_hit.len() as u32;
    let stage = RayTracingShaderStageDefine {
      logic: Box::new(ray_logic),
      user_defined_payload_input_ty: P::sized_ty(),
    };
    self.miss_hit.push(stage);
    ShaderHandle(idx, RayTracingShaderStage::Miss)
  }

  pub fn register_ray_closest_hit<P: ShaderSizedValueNodeType>(
    &mut self,
    ray_logic: impl TraceOperator<()> + 'static,
  ) -> ShaderHandle {
    let idx = self.closest_hit.len() as u32;
    let stage = RayTracingShaderStageDefine {
      logic: Box::new(ray_logic),
      user_defined_payload_input_ty: P::sized_ty(),
    };
    self.closest_hit.push(stage);
    ShaderHandle(idx, RayTracingShaderStage::ClosestHit)
  }

  pub fn register_ray_intersection(
    &mut self,
    builder: impl Fn(&RayIntersectCtx, &dyn IntersectionReporter) + 'static,
  ) -> ShaderHandle {
    let idx = self.intersection.len() as u32;
    self.intersection.push(Arc::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::Intersection)
  }

  pub fn register_ray_any_hit(
    &mut self,
    builder: impl Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior> + 'static,
  ) -> ShaderHandle {
    let idx = self.any_hit.len() as u32;
    self.any_hit.push(Arc::new(builder));
    ShaderHandle(idx, RayTracingShaderStage::AnyHit)
  }
}
