use crate::{BindGroupLayoutProvider, GeometryProvider, TargetStates, WGPURenderer};
use std::{borrow::Cow, sync::Arc};

/// Descriptor of the shader input
#[derive(Clone)]
pub struct PipelineShaderInterfaceInfo {
  bindgroup_layouts: Vec<Arc<wgpu::BindGroupLayout>>,
  vertex_state: Option<wgpu::VertexStateDescriptor<'static>>,
  primitive_topology: wgpu::PrimitiveTopology,
  pub preferred_target_states: TargetStates,
}

impl PipelineShaderInterfaceInfo {
  pub fn new() -> Self {
    Self {
      bindgroup_layouts: Vec::new(),
      vertex_state: None,
      primitive_topology: wgpu::PrimitiveTopology::TriangleList,
      preferred_target_states: TargetStates::default(),
    }
  }

  pub fn binding_group<T: BindGroupLayoutProvider>(
    &mut self,
    layout: Arc<wgpu::BindGroupLayout>,
  ) -> &mut Self {
    self.bindgroup_layouts.push(layout.clone());
    self
  }

  pub fn geometry<T: GeometryProvider>(&mut self) -> &mut Self {
    self.vertex_state = Some(T::get_geometry_vertex_state_descriptor());
    self.primitive_topology = T::get_primitive_topology();
    self
  }
}

pub struct PipelineBuilder {
  vertex_shader: Vec<u32>,
  frag_shader: Vec<u32>,
  pub shader_interface_info: PipelineShaderInterfaceInfo,
  pub target_states: TargetStates,
  pub rasterization: wgpu::RasterizationStateDescriptor,
}

impl AsMut<Self> for PipelineBuilder {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl PipelineBuilder {
  pub fn new(
    vertex_shader: Vec<u32>,
    frag_shader: Vec<u32>,
    shader_interface_info: PipelineShaderInterfaceInfo,
  ) -> Self {
    Self {
      vertex_shader,
      frag_shader,
      shader_interface_info,
      rasterization: wgpu::RasterizationStateDescriptor {
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: wgpu::CullMode::None,
        depth_bias: 0,
        depth_bias_slope_scale: 0.0,
        depth_bias_clamp: 0.0,
        clamp_depth: false,
      },
      target_states: TargetStates::default(),
    }
  }

  pub fn target_states(&mut self, states: &TargetStates) -> &mut Self {
    self.target_states = states.clone();
    self
  }

  pub fn build(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
    let bind_group_layouts: Vec<_> = self
      .shader_interface_info
      .bindgroup_layouts
      .iter()
      .map(|l| l.as_ref())
      .collect();
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      push_constant_ranges: &[],
      bind_group_layouts: &bind_group_layouts,
    });

    // Create the render pipeline
    let vs_module_source = wgpu::ShaderModuleSource::SpirV(Cow::Borrowed(&self.vertex_shader));
    let fs_module_source = wgpu::ShaderModuleSource::SpirV(Cow::Borrowed(&self.frag_shader));
    let vs_module = device.create_shader_module(vs_module_source);
    let fs_module = device.create_shader_module(fs_module_source);

    let pipeline_des = wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),

      vertex_stage: wgpu::ProgrammableStageDescriptor {
        module: &vs_module,
        entry_point: "main",
      },
      fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
        module: &fs_module,
        entry_point: "main",
      }),

      color_states: &self.target_states.color_states,
      depth_stencil_state: self.target_states.depth_state.to_owned(),

      primitive_topology: self.shader_interface_info.primitive_topology,
      vertex_state: self.shader_interface_info.vertex_state.to_owned().unwrap(),
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
      rasterization_state: Some(self.rasterization.clone()),
    };

    device.create_render_pipeline(&pipeline_des)
  }
}
