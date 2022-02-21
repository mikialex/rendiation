use crate::*;

use shadergraph::*;
pub mod container;
pub use container as c;

#[derive(Clone)]
pub struct GPURenderPipeline {
  pub inner: Rc<GPURenderPipelineInner>,
}

impl GPURenderPipeline {
  pub fn new(pipeline: wgpu::RenderPipeline, bg_layouts: Vec<GPUBindGroupLayout>) -> Self {
    let inner = GPURenderPipelineInner {
      pipeline,
      bg_layouts,
    };
    Self {
      inner: Rc::new(inner),
    }
  }

  pub fn get_layout(&self, sb: SemanticBinding) -> &GPUBindGroupLayout {
    let index = sb.binding_index();
    self.bg_layouts.get(index).unwrap()
  }
}

pub struct GPURenderPipelineInner {
  pub pipeline: wgpu::RenderPipeline,
  pub bg_layouts: Vec<GPUBindGroupLayout>,
}

impl Deref for GPURenderPipeline {
  type Target = GPURenderPipelineInner;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub fn create_bindgroup_layout_by_node_ty<'a>(
  device: &GPUDevice,
  iter: impl Iterator<Item = (&'a ShaderValueType, wgpu::ShaderStages)>,
) -> GPUBindGroupLayout {
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

  device.create_and_cache_bindgroup_layout(entries.as_ref())
}

impl GPUDevice {
  pub fn build_pipeline_by_shadergraph(
    &self,
    builder: ShaderGraphRenderPipelineBuilder,
  ) -> Result<GPURenderPipeline, ShaderGraphBuildError> {
    let compile_result = builder.build(WGSL)?;

    let ShaderGraphCompileResult {
      shader,
      bindings,
      vertex_layouts,
      primitive_state,
      color_states,
      depth_stencil,
      multisample,
      target,
    } = compile_result;

    let WGSLShaderSource { vertex, fragment } = shader;

    let vertex = self.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(vertex.as_str())),
    });
    let fragment = self.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(fragment.as_str())),
    });

    let layouts: Vec<_> = bindings
      .bindings
      .iter()
      .map(|binding| {
        let iter = binding.bindings.iter().map(|(ty, vis)| {
          let visibility = match vis.get() {
            ShaderStageVisibility::Vertex => wgpu::ShaderStages::VERTEX,
            ShaderStageVisibility::Fragment => wgpu::ShaderStages::FRAGMENT,
            ShaderStageVisibility::Both => wgpu::ShaderStages::VERTEX_FRAGMENT,
            ShaderStageVisibility::None => wgpu::ShaderStages::NONE,
          };
          (ty, visibility)
        });

        create_bindgroup_layout_by_node_ty(self, iter)
      })
      .collect();

    let layouts_ref: Vec<_> = layouts.iter().map(|l| l.inner.as_ref()).collect();

    let pipeline_layout = self.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: layouts_ref.as_slice(),
      push_constant_ranges: &[],
    });

    let vertex_buffers: Vec<_> = vertex_layouts.iter().map(convert_vertex_layout).collect();

    let pipeline = self.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &vertex,
        entry_point: target.vertex_entry_name(),
        buffers: vertex_buffers.as_slice(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &fragment,
        entry_point: target.fragment_entry_name(),
        targets: color_states.as_slice(),
      }),
      primitive: primitive_state,
      depth_stencil,
      multisample,
      multiview: None,
    });

    Ok(GPURenderPipeline::new(pipeline, layouts))
  }
}

pub fn convert_vertex_layout(layout: &ShaderGraphVertexBufferLayout) -> wgpu::VertexBufferLayout {
  wgpu::VertexBufferLayout {
    array_stride: layout.array_stride,
    step_mode: layout.step_mode,
    attributes: layout.attributes.as_slice(),
  }
}
