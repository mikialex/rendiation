use std::{borrow::Cow, rc::Rc};

pub struct PipelineBuilder {
  name: String,
  shader_source: String,
  layouts: Vec<Rc<wgpu::BindGroupLayout>>,
  targets: Vec<wgpu::ColorTargetState>,
  depth_stencil: Option<wgpu::DepthStencilState>,
  vertex_buffers: Vec<wgpu::VertexBufferLayout<'static>>,
  primitive_state: wgpu::PrimitiveState,
}

impl PipelineBuilder {
  //

  pub fn with_layout(&mut self, layout: &Rc<wgpu::BindGroupLayout>) -> &mut Self {
    self.layouts.push(layout.clone());
    self
  }

  pub fn with_topology(&mut self, topology: wgpu::PrimitiveTopology) -> &mut Self {
    self.primitive_state.topology = topology;
    self
  }

  pub fn build(self, device: wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: self.name.as_str().into(),
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(self.shader_source.as_str())),
    });

    let layouts: Vec<_> = self.layouts.iter().map(|l| l.as_ref()).collect();

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: layouts.as_slice(),
      push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: "vs_main",
        buffers: self.vertex_buffers.as_slice(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: self.targets.as_slice(),
      }),
      primitive: wgpu::PrimitiveState {
        cull_mode: None,
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: self.depth_stencil,
      multisample: wgpu::MultisampleState::default(),
    })
  }
}
