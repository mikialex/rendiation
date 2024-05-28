use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DeviceDrawStatistics {
  /// Amount of times the vertex shader is ran. Accounts for the vertex cache when doing indexed
  /// rendering.
  pub vertex_shader_invocations: u64,
  /// Amount of times the clipper is invoked. This is also the amount of triangles output by the
  /// vertex shader.
  pub clipper_invocations: u64,
  /// Amount of primitives that are not culled by the clipper. This is the amount of triangles that
  /// are actually on screen and will be rasterized and rendered.
  pub clipper_primitives_out: u64,
  /// Amount of times the fragment shader is ran. Accounts for fragment shaders running in 2x2
  /// blocks in order to get derivatives.
  pub fragment_shader_invocations: u64,
  /// Amount of times a compute shader is invoked. This will be equivalent to the dispatch count
  /// times the workgroup size.
  pub compute_shader_invocations: u64,
}

impl std::ops::Add<Self> for DeviceDrawStatistics {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      vertex_shader_invocations: self.vertex_shader_invocations + rhs.vertex_shader_invocations,
      clipper_invocations: self.clipper_invocations + rhs.clipper_invocations,
      clipper_primitives_out: self.clipper_primitives_out + rhs.clipper_primitives_out,
      fragment_shader_invocations: self.fragment_shader_invocations
        + rhs.fragment_shader_invocations,
      compute_shader_invocations: self.compute_shader_invocations + rhs.compute_shader_invocations,
    }
  }
}

/// should not nesting
pub struct PipelineQuery {
  query_set: gpu::QuerySet,
}

impl PipelineQuery {
  pub fn start(device: &GPUDevice, pass: &mut GPURenderPass) -> Self {
    let query_set = device.create_query_set(&QuerySetDescriptor {
      label: "pipeline-query".into(),
      ty: QueryType::PipelineStatistics(PipelineStatisticsTypes::all()),
      count: 1,
    });

    pass.begin_pipeline_statistics_query(&query_set, 0);
    Self { query_set }
  }

  /// should use same render pass
  pub fn end(self, pass: &mut GPURenderPass) -> PipelineQueryResult {
    pass.end_pipeline_statistics_query();
    PipelineQueryResult {
      result: self.query_set,
    }
  }
}

pub struct PipelineQueryResult {
  result: gpu::QuerySet,
}

impl PipelineQueryResult {
  pub fn read_back(
    self,
    device: &GPUDevice,
    encoder: &mut GPUCommandEncoder,
  ) -> impl Future<Output = Option<DeviceDrawStatistics>> + Unpin {
    read_back_query::<DeviceDrawStatistics>(&self.result, 0..1, device, encoder)
  }
}
