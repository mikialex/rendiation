use __core::num::NonZeroU32;
use rendiation_shader_api::*;

use crate::*;
pub mod container;
pub use container as c;

#[derive(Clone)]
pub struct GPURenderPipeline {
  pub inner: Arc<GPURenderPipelineInner>,
}

impl GPURenderPipeline {
  fn new(pipeline: gpu::RenderPipeline, bg_layouts: Vec<GPUBindGroupLayout>) -> Self {
    let inner = GPURenderPipelineInner {
      pipeline,
      bg_layouts,
    };
    Self {
      inner: Arc::new(inner),
    }
  }

  pub fn get_layout(&self, index: usize) -> &GPUBindGroupLayout {
    self.bg_layouts.get(index).unwrap()
  }
}

pub struct GPURenderPipelineInner {
  pub pipeline: gpu::RenderPipeline,
  pub bg_layouts: Vec<GPUBindGroupLayout>,
}

impl Deref for GPURenderPipeline {
  type Target = GPURenderPipelineInner;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub fn map_shader_value_ty_to_binding_layout_type(
  v: ShaderBindingDescriptor,
  id: usize,
) -> gpu::BindGroupLayoutEntry {
  use ShaderValueSingleType::*;
  let ty = v
    .ty
    .visit_single(|ty| match *ty {
      Sized(_) => gpu::BindingType::Buffer {
        ty: if v.should_as_storage_buffer_if_is_buffer_like {
          gpu::BufferBindingType::Storage { read_only: true }
        } else {
          gpu::BufferBindingType::Uniform
        },
        has_dynamic_offset: false,
        // min_binding_size: gpu::BufferSize::new(std::mem::size_of::<T>() as u64), // todo
        min_binding_size: None,
      },
      Unsized(_) => gpu::BindingType::Buffer {
        ty: gpu::BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        // min_binding_size: gpu::BufferSize::new(std::mem::size_of::<T>() as u64), // todo
        min_binding_size: None,
      },
      Sampler(ty) => gpu::BindingType::Sampler(ty),
      Texture {
        dimension,
        sample_type,
      } => gpu::BindingType::Texture {
        multisampled: false,
        sample_type,
        view_dimension: dimension,
      },
      CompareSampler => gpu::BindingType::Sampler(gpu::SamplerBindingType::Comparison),
    })
    .unwrap();

  let count = match v.ty {
    ShaderValueType::BindingArray { count, .. } => Some(NonZeroU32::new(count as u32).unwrap()),
    _ => None,
  };

  gpu::BindGroupLayoutEntry {
    binding: id as u32,
    visibility: gpu::ShaderStages::VERTEX_FRAGMENT,
    ty,
    count,
  }
}

pub fn create_bindgroup_layout_by_node_ty<'a>(
  device: &GPUDevice,
  iter: impl Iterator<Item = &'a ShaderBindingDescriptor>,
) -> GPUBindGroupLayout {
  let entries: Vec<_> = iter
    .enumerate()
    .map(|(i, ty)| map_shader_value_ty_to_binding_layout_type(*ty, i))
    .collect();

  device.create_and_cache_bindgroup_layout(entries.as_ref())
}

impl GPUDevice {
  pub fn build_pipeline_by_shadergraph(
    &self,
    builder: ShaderGraphRenderPipelineBuilder,
  ) -> Result<GPURenderPipeline, ShaderGraphBuildError> {
    let log_result = builder.log_result;
    let compile_result = builder.build()?;

    let ShaderGraphCompileResult {
      vertex_shader: (vertex_shader, vertex_entry),
      frag_shader: (frag_shader, frag_entry),
      bindings,
      vertex_layouts,
      primitive_state,
      color_states,
      depth_stencil,
      multisample,
    } = compile_result;

    if log_result {
      println!();
      println!("=== rendiation_shader_api build result ===");
      println!("vertex shader: ");
      println!("{vertex_shader}");
      println!("fragment shader: ");
      println!("{frag_shader}");
    }

    let vertex = self.create_shader_module(gpu::ShaderModuleDescriptor {
      label: None,
      source: gpu::ShaderSource::Wgsl(Cow::Borrowed(vertex_shader.as_str())),
    });
    let fragment = self.create_shader_module(gpu::ShaderModuleDescriptor {
      label: None,
      source: gpu::ShaderSource::Wgsl(Cow::Borrowed(frag_shader.as_str())),
    });

    let binding = &bindings.bindings;
    let last_empty_count = binding
      .iter()
      .rev()
      .take_while(|l| l.bindings.is_empty())
      .count();

    let layouts: Vec<_> = binding
      .get(0..binding.len() - last_empty_count)
      .unwrap()
      .iter()
      .map(|b| create_bindgroup_layout_by_node_ty(self, b.bindings.iter().map(|e| &e.desc)))
      .collect();

    let layouts_ref: Vec<_> = layouts.iter().map(|l| l.inner.as_ref()).collect();

    let pipeline_layout = self.create_pipeline_layout(&gpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: layouts_ref.as_slice(),
      push_constant_ranges: &[],
    });

    let vertex_buffers: Vec<_> = vertex_layouts.iter().map(convert_vertex_layout).collect();

    let pipeline = self.create_render_pipeline(&gpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: gpu::VertexState {
        module: &vertex,
        entry_point: &vertex_entry,
        buffers: vertex_buffers.as_slice(),
      },
      fragment: Some(gpu::FragmentState {
        module: &fragment,
        entry_point: &frag_entry,
        targets: color_states
          .iter()
          .map(|s| Some(s.clone()))
          .collect::<Vec<_>>()
          .as_slice(),
      }),
      primitive: primitive_state,
      depth_stencil,
      multisample,
      multiview: None,
    });

    Ok(GPURenderPipeline::new(pipeline, layouts))
  }
}

pub fn convert_vertex_layout(layout: &ShaderGraphVertexBufferLayout) -> gpu::VertexBufferLayout {
  gpu::VertexBufferLayout {
    array_stride: layout.array_stride,
    step_mode: layout.step_mode,
    attributes: layout.attributes.as_slice(),
  }
}
