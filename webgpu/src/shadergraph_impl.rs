use std::borrow::Cow;

use shadergraph::ShaderGraphCompileResult;

pub fn build_shader(
  compile_result: &ShaderGraphCompileResult,
  device: &wgpu::Device,
) -> wgpu::RenderPipeline {
  let ShaderGraphCompileResult {
    vertex_shader,
    frag_shader,
    states,
    bindings,
  } = compile_result;

  let vertex_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(vertex_shader.as_str())),
  });
  let frag_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(frag_shader.as_str())),
  });

  let layouts: Vec<_> = bindings
    .bindings
    .iter()
    .map(|binding| {
      let entries: Vec<_> = binding
        .bindings
        .iter()
        .enumerate()
        .map(|(i, (binding, _))| {
          let ty = match binding.ty {
            shadergraph::ShaderValueType::Fixed(_) => wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              // min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<T>() as u64), // todo
              min_binding_size: None,
            },
            shadergraph::ShaderValueType::Sampler => {
              wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
            }
            shadergraph::ShaderValueType::Texture => wgpu::BindingType::Texture {
              multisampled: false,
              sample_type: wgpu::TextureSampleType::Float { filterable: true },
              view_dimension: wgpu::TextureViewDimension::D2,
            },
            shadergraph::ShaderValueType::Never => unreachable!(),
          };

          let mut visibility = wgpu::ShaderStages::NONE;
          if binding.node_vertex.is_some() {
            visibility.set(wgpu::ShaderStages::VERTEX, true);
          }
          if binding.node_fragment.is_some() {
            visibility.set(wgpu::ShaderStages::FRAGMENT, true);
          }

          wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            visibility,
            ty,
            count: None,
          }
        })
        .collect();

      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: entries.as_ref(),
      })
    })
    .collect();

  let layouts: Vec<_> = layouts.iter().collect();

  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: layouts.as_slice(),
    push_constant_ranges: &[],
  });

  // let vertex_buffers: Vec<_> = self.vertex_buffers.iter().map(|v| v.as_raw()).collect();

  // device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
  //   label: None,
  //   layout: Some(&pipeline_layout),
  //   vertex: wgpu::VertexState {
  //     module: &vertex_shader,
  //     entry_point: self.active_vertex_entry.as_str(),
  //     buffers: vertex_buffers.as_slice(),
  //   },
  //   fragment: Some(wgpu::FragmentState {
  //     module: &frag_shader,
  //     entry_point: self.active_fragment_entry.as_str(),
  //     targets: self.targets.as_slice(),
  //   }),
  //   primitive: self.primitive_state,
  //   depth_stencil: self.depth_stencil.clone(),
  //   multisample: self.multisample,
  //   multiview: None,
  // })
  todo!()
}
