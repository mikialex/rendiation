use crate::*;

pub struct RayGenShaderCtx {
  launch_id: Node<Vec3<u32>>,
  launch_size: Node<Vec3<u32>>,
}

struct AdhocStateBuilder {
  states: Vec<ShaderSizedValueType>,
}

impl RayGenShaderCtx {
  pub fn call_trace_ray(&mut self, trace: ShaderRayTraceCall) {
    //
  }
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

pub struct RayAnyHitCtx {
  //
}

pub struct RayIntersectCtx {
  //
}

impl RayIntersectCtx {
  /// Invokes the current hit shader once an intersection shader has determined
  /// that a ray intersection has occurred. If the intersection occurred within
  /// the current ray interval, the any-hit shader corresponding to the current
  /// intersection shader is invoked. If the intersection is not ignored in the
  /// any-hit shader, <hitT> is committed as the new gl_RayTmaxEXT value of the
  /// current ray, <hitKind> is committed as the new value for gl_HitKindEXT, and
  /// true is returned. If either of those checks fails, then false is returned.
  /// If the value of <hitT> falls outside the current ray interval, the hit is
  /// rejected and false is returned.
  ///
  /// https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_tracing.txt#L954
  pub fn report_intersection(&self, hit_t: Node<f32>, hit_kind: Node<u32>) -> Node<bool> {
    todo!()
  }
}

pub struct RayMissCtx {
  //
}
