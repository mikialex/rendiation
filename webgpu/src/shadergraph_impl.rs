use std::borrow::Cow;

use shadergraph::*;

use crate::{GPUDevice, GPURenderPipeline};

pub fn create_bindgroup_layout_by_node_ty<'a>(
  device: &GPUDevice,
  iter: impl Iterator<Item = (&'a ShaderValueType, wgpu::ShaderStages)>,
) -> wgpu::BindGroupLayout {
  let entries: Vec<_> = iter
    .enumerate()
    .map(|(i, (ty, visibility))| {
      let ty = match ty {
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
}

pub fn build_pipeline(
  builder: &dyn ShaderGraphProvider,
  device: &GPUDevice,
) -> Result<GPURenderPipeline, ShaderGraphBuildError> {
  let target = WGSL;
  let compile_result = build_shader(builder, &target)?;

  let ShaderGraphCompileResult {
    vertex_shader,
    frag_shader,
    bindings,
    vertex_layouts,
    primitive_state,
    color_states,
    depth_stencil,
    multisample,
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
      let iter = binding.bindings.iter().map(|(binding, _)| {
        let mut visibility = wgpu::ShaderStages::NONE;
        if binding.node_vertex.is_some() {
          visibility.set(wgpu::ShaderStages::VERTEX, true);
        }
        if binding.node_fragment.is_some() {
          visibility.set(wgpu::ShaderStages::FRAGMENT, true);
        }
        (&binding.ty, visibility)
      });

      create_bindgroup_layout_by_node_ty(device, iter)
    })
    .collect();

  let layouts_ref: Vec<_> = layouts.iter().collect();

  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: layouts_ref.as_slice(),
    push_constant_ranges: &[],
  });

  let vertex_buffers: Vec<_> = vertex_layouts.iter().map(convert_vertex_layout).collect();

  let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &vertex_shader,
      entry_point: target.vertex_entry_name(),
      buffers: vertex_buffers.as_slice(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &frag_shader,
      entry_point: target.fragment_entry_name(),
      targets: color_states.as_slice(),
    }),
    primitive: primitive_state,
    depth_stencil,
    multisample,
    multiview: None,
  });

  // Ok(GPURenderPipeline {
  //   pipeline,
  //   bg_layouts: layouts,
  // })
  todo!()
}

pub fn convert_vertex_layout(layout: &ShaderGraphVertexBufferLayout) -> wgpu::VertexBufferLayout {
  wgpu::VertexBufferLayout {
    array_stride: layout.array_stride,
    step_mode: layout.step_mode,
    attributes: layout.attributes.as_slice(),
  }
}
