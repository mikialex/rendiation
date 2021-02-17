use rendiation_ral::{PipelineShaderInterfaceInfo, PrimitiveState, TargetStates};
use std::borrow::Cow;

use crate::BindGroupLayoutCache;

pub struct PipelineBuilder {
  vertex_shader: Vec<u32>,
  frag_shader: Vec<u32>,
  pub shader_interface_info: PipelineShaderInterfaceInfo,
  pub target_states: TargetStates,
  pub primitive_states: wgpu::PrimitiveState,
}

impl AsMut<Self> for PipelineBuilder {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl PipelineBuilder {
  pub fn new(
    vertex_shader: &Vec<u32>,
    frag_shader: &Vec<u32>,
    shader_interface_info: PipelineShaderInterfaceInfo,
  ) -> Self {
    Self {
      vertex_shader: vertex_shader.clone(),
      frag_shader: frag_shader.clone(),
      shader_interface_info,
      primitive_states: PrimitiveState::default(),
      target_states: TargetStates::default(),
    }
  }

  pub fn target_states(&mut self, states: &TargetStates) -> &mut Self {
    self.target_states = states.clone();
    self
  }

  pub fn build(&self, device: &wgpu::Device, cache: &BindGroupLayoutCache) -> wgpu::RenderPipeline {
    let bind_group_layouts: Vec<_> = self
      .shader_interface_info
      .bindgroup_layouts
      .iter()
      .map(|desc| cache.get_bindgroup_layout(desc, device))
      .collect();
    let bind_group_layouts: Vec<_> = bind_group_layouts.iter().map(|d| d.as_ref()).collect();

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      push_constant_ranges: &[],
      bind_group_layouts: bind_group_layouts.as_slice(),
    });

    // Create the render pipeline
    let vs_module_source = wgpu::ShaderSource::SpirV(Cow::Borrowed(&self.vertex_shader));
    let fs_module_source = wgpu::ShaderSource::SpirV(Cow::Borrowed(&self.frag_shader));
    let vs = wgpu::ShaderModuleDescriptor {
      label: None,
      source: vs_module_source,
      flags: wgpu::ShaderFlags::empty(),
    };
    let fs = wgpu::ShaderModuleDescriptor {
      label: None,
      source: fs_module_source,
      flags: wgpu::ShaderFlags::empty(),
    };
    let vs_module = device.create_shader_module(&vs);
    let fs_module = device.create_shader_module(&fs);

    todo!()

    // // because of VertexBufferDescriptor stuff not included in ral core, we should do an conversion
    // let vertex_buffer_des: Vec<wgpu::VertexBufferDescriptor> = self
    //   .shader_interface_info
    //   .vertex_state
    //   .to_owned()
    //   .map(|d| {
    //     d.vertex_buffers
    //       .iter()
    //       .map(|de| wgpu::VertexBufferDescriptor {
    //         stride: de.stride,
    //         step_mode: de.step_mode,
    //         attributes: de.attributes,
    //       })
    //       .collect()
    //   })
    //   .unwrap();

    // let wgpu_vertex_state = self
    //   .shader_interface_info
    //   .vertex_state
    //   .to_owned()
    //   .map(|d| wgpu::VertexStateDescriptor {
    //     index_format: d.index_format,
    //     vertex_buffers: &vertex_buffer_des,
    //   })
    //   .unwrap();

    // let pipeline_des = wgpu::RenderPipelineDescriptor {
    //   label: None,
    //   layout: Some(&pipeline_layout),

    //   // vertex_stage: wgpu::ProgrammableStageDescriptor {
    //   //   module: &vs_module,
    //   //   entry_point: "main",
    //   // },
    //   // fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
    //   //   module: &fs_module,
    //   //   entry_point: "main",
    //   // }),

    //   // color_states: &self.target_states.color_states,
    //   // depth_stencil_state: self.target_states.depth_state.to_owned(),

    //   // primitive_topology: self.shader_interface_info.primitive_topology,
    //   // vertex_state: wgpu_vertex_state,
    //   // sample_count: 1,
    //   // sample_mask: !0,
    //   // alpha_to_coverage_enabled: false,
    //   // rasterization_state: Some(self.rasterization.clone()),
    //   vertex: (),
    //   primitive: (),
    //   depth_stencil: (),
    //   multisample: (),
    //   fragment: Some(FragmentState {}),
    // };

    // device.create_render_pipeline(&pipeline_des)
  }
}
