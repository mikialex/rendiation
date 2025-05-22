use core::num::NonZeroU32;

use rendiation_shader_backend_naga::ShaderAPINagaImpl;

mod container;
pub use container::*;

pub type GPURenderPipeline = GPUPipeline<wgpu::RenderPipeline>;
pub type GPUComputePipeline = GPUPipeline<wgpu::ComputePipeline>;

pub struct GPUPipeline<T> {
  pub inner: Arc<GPUPipelineImpl<T>>,
}

impl<T> Deref for GPUPipeline<T> {
  type Target = GPUPipelineImpl<T>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> Clone for GPUPipeline<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> GPUPipeline<T> {
  fn new(
    pipeline: T,
    raw_bg_layouts: Vec<GPUBindGroupLayout>,
    bg_layouts: Vec<Vec<ShaderBindingDescriptor>>,
  ) -> Self {
    let inner = GPUPipelineImpl {
      pipeline,
      bg_layouts,
      raw_bg_layouts,
    };
    Self {
      inner: Arc::new(inner),
    }
  }
}

pub struct GPUPipelineImpl<T> {
  pub pipeline: T,
  pub raw_bg_layouts: Vec<GPUBindGroupLayout>,
  pub bg_layouts: Vec<Vec<ShaderBindingDescriptor>>,
}

impl<T> Deref for GPUPipelineImpl<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.pipeline
  }
}

pub fn map_shader_value_ty_to_binding_layout_type(
  v: &ShaderBindingDescriptor,
  id: usize,
  visibility: ShaderStages,
) -> gpu::BindGroupLayoutEntry {
  use ShaderValueSingleType::*;
  let ty = v
    .ty
    .visit_single(|ty| match *ty {
      Sized(_) => gpu::BindingType::Buffer {
        ty: if v.should_as_storage_buffer_if_is_buffer_like {
          gpu::BufferBindingType::Storage {
            read_only: !v.writeable_if_storage,
          }
        } else {
          gpu::BufferBindingType::Uniform
        },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      Unsized(_) => gpu::BindingType::Buffer {
        ty: gpu::BufferBindingType::Storage {
          read_only: !v.writeable_if_storage,
        },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      Sampler(ty) => gpu::BindingType::Sampler(ty),
      Texture {
        dimension,
        sample_type,
        multi_sampled,
      } => gpu::BindingType::Texture {
        multisampled: multi_sampled,
        sample_type,
        view_dimension: dimension,
      },
      StorageTexture {
        dimension,
        format,
        access,
      } => gpu::BindingType::StorageTexture {
        access: match access {
          rendiation_shader_api::StorageTextureAccess::Load => gpu::StorageTextureAccess::ReadOnly,
          rendiation_shader_api::StorageTextureAccess::Store => {
            gpu::StorageTextureAccess::WriteOnly
          }
          rendiation_shader_api::StorageTextureAccess::LoadStore => {
            gpu::StorageTextureAccess::ReadWrite
          }
        },
        format: match format {
          StorageFormat::R8Unorm => TextureFormat::R8Unorm,
          StorageFormat::R8Snorm => TextureFormat::R8Snorm,
          StorageFormat::R8Uint => TextureFormat::R8Uint,
          StorageFormat::R8Sint => TextureFormat::R8Sint,
          StorageFormat::R16Uint => TextureFormat::R16Uint,
          StorageFormat::R16Sint => TextureFormat::R16Sint,
          StorageFormat::R16Float => TextureFormat::R16Float,
          StorageFormat::Rg8Unorm => TextureFormat::Rg8Unorm,
          StorageFormat::Rg8Snorm => TextureFormat::Rg8Snorm,
          StorageFormat::Rg8Uint => TextureFormat::Rg8Uint,
          StorageFormat::Rg8Sint => TextureFormat::Rg8Sint,
          StorageFormat::R32Uint => TextureFormat::R32Uint,
          StorageFormat::R32Sint => TextureFormat::R32Sint,
          StorageFormat::R32Float => TextureFormat::R32Float,
          StorageFormat::Rg16Uint => TextureFormat::Rg16Uint,
          StorageFormat::Rg16Sint => TextureFormat::Rg16Sint,
          StorageFormat::Rg16Float => TextureFormat::Rg16Float,
          StorageFormat::Rgba8Unorm => TextureFormat::Rgba8Unorm,
          StorageFormat::Rgba8Snorm => TextureFormat::Rgba8Snorm,
          StorageFormat::Rgba8Uint => TextureFormat::Rgba8Uint,
          StorageFormat::Rgba8Sint => TextureFormat::Rgba8Sint,
          StorageFormat::Bgra8Unorm => TextureFormat::Bgra8Unorm,
          StorageFormat::Rgb10a2Uint => TextureFormat::Rgb10a2Uint,
          StorageFormat::Rgb10a2Unorm => TextureFormat::Rgb10a2Unorm,
          StorageFormat::Rg32Uint => TextureFormat::Rg32Uint,
          StorageFormat::Rg32Sint => TextureFormat::Rg32Sint,
          StorageFormat::Rg32Float => TextureFormat::Rg32Float,
          StorageFormat::Rgba16Uint => TextureFormat::Rgba16Uint,
          StorageFormat::Rgba16Sint => TextureFormat::Rgba16Sint,
          StorageFormat::Rgba16Float => TextureFormat::Rgba16Float,
          StorageFormat::Rgba32Uint => TextureFormat::Rgba32Uint,
          StorageFormat::Rgba32Sint => TextureFormat::Rgba32Sint,
          StorageFormat::Rgba32Float => TextureFormat::Rgba32Float,
          StorageFormat::R16Unorm => TextureFormat::R16Unorm,
          StorageFormat::R16Snorm => TextureFormat::R16Snorm,
          StorageFormat::Rg16Unorm => TextureFormat::Rg16Unorm,
          StorageFormat::Rg16Snorm => TextureFormat::Rg16Snorm,
          StorageFormat::Rgba16Unorm => TextureFormat::Rgba16Unorm,
          StorageFormat::Rgba16Snorm => TextureFormat::Rgba16Snorm,
        },
        view_dimension: dimension,
      },
      AccelerationStructure => gpu::BindingType::AccelerationStructure,
      RayQuery => unreachable!(),
    })
    .unwrap();

  let count = match v.ty {
    ShaderValueType::BindingArray { count, .. } => Some(NonZeroU32::new(count as u32).unwrap()),
    _ => None,
  };

  if visibility == ShaderStages::empty() {
    dbg!(&v.ty);
  }

  assert!(visibility != ShaderStages::empty());

  gpu::BindGroupLayoutEntry {
    binding: id as u32,
    visibility,
    ty,
    count,
  }
}

impl GPUDevice {
  pub fn create_shader_module_by_shader_api(
    &self,
    naga_module: naga::Module,
    log_result: bool,
  ) -> wgpu::ShaderModule {
    if log_result {
      println!();
      println!("=== rendiation_shader_api build result ===");

      let shader_str = convert_module_by_wgsl(&naga_module, naga::valid::ValidationFlags::all());
      println!("{shader_str}",);

      println!("=== result output finished ===");
    }

    self.create_shader_module(gpu::ShaderModuleDescriptor {
      label: None,
      source: gpu::ShaderSource::Naga(Cow::Owned(naga_module)),
    })
  }

  pub fn build_pipeline_by_shader_api(
    &self,
    builder: ShaderRenderPipelineBuilder,
  ) -> Result<GPURenderPipeline, ShaderBuildError> {
    let log_result = builder.log_result;
    let compile_result = builder.build()?;

    let GraphicsShaderCompileResult {
      vertex_shader: (vertex_entry, vertex_shader),
      frag_shader: (frag_entry, frag_shader),
      bindings,
      vertex_layouts,
      primitive_state,
      color_states,
      depth_stencil,
      multisample,
    } = compile_result;

    let naga_vertex = *vertex_shader.downcast::<naga::Module>().unwrap();
    let naga_fragment = *frag_shader.downcast::<naga::Module>().unwrap();

    let vertex = self.create_shader_module_by_shader_api(naga_vertex, log_result);
    let fragment = self.create_shader_module_by_shader_api(naga_fragment, log_result);

    let (raw_layouts, layouts, pipeline_layout) = create_layouts(self, &bindings);

    let vertex_buffers: Vec<_> = vertex_layouts.iter().map(convert_vertex_layout).collect();

    let pipeline = self.create_render_pipeline(&gpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: gpu::VertexState {
        module: &vertex,
        entry_point: Some(&vertex_entry),
        buffers: vertex_buffers.as_slice(),
        compilation_options: Default::default(),
      },
      fragment: Some(gpu::FragmentState {
        module: &fragment,
        entry_point: Some(&frag_entry),
        targets: color_states
          .iter()
          .map(|s| Some(s.clone()))
          .collect::<Vec<_>>()
          .as_slice(),
        compilation_options: Default::default(),
      }),
      primitive: primitive_state,
      depth_stencil,
      multisample,
      multiview: None,
      cache: None,
    });

    Ok(GPUPipeline::new(pipeline, raw_layouts, layouts))
  }
}

fn create_layouts(
  device: &GPUDevice,
  builder: &ShaderBindGroupBuilder,
) -> (
  Vec<GPUBindGroupLayout>,
  Vec<Vec<ShaderBindingDescriptor>>,
  wgpu::PipelineLayout,
) {
  let binding = &builder.bindings;
  let last_empty_count = binding
    .iter()
    .rev()
    .take_while(|l| l.bindings.is_empty())
    .count();

  let raw_layouts: Vec<_> = binding
    .get(0..binding.len() - last_empty_count)
    .unwrap()
    .iter()
    .map(|b| {
      device.create_and_cache_bindgroup_layout(b.bindings.iter().map(|e| (&e.desc, e.visibility)))
    })
    .collect();

  let layouts: Vec<_> = binding
    .get(0..binding.len() - last_empty_count)
    .unwrap()
    .iter()
    .map(|b| b.bindings.iter().map(|e| e.desc.clone()).collect())
    .collect();

  let layouts_ref: Vec<_> = raw_layouts.iter().map(|l| &l.inner).collect();

  let pipeline_layout = device.create_pipeline_layout(&gpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: layouts_ref.as_slice(),
    push_constant_ranges: &[],
  });
  (raw_layouts, layouts, pipeline_layout)
}

fn convert_module_by_wgsl(module: &naga::Module, v: naga::valid::ValidationFlags) -> String {
  use naga::back::wgsl;

  let info = naga::valid::Validator::new(v, naga::valid::Capabilities::all())
    .validate(module)
    .unwrap();

  wgsl::write_string(module, &info, wgsl::WriterFlags::empty()).unwrap()
}

pub fn convert_vertex_layout(layout: &ShaderVertexBufferLayout) -> gpu::VertexBufferLayout {
  gpu::VertexBufferLayout {
    array_stride: layout.array_stride,
    step_mode: layout.step_mode,
    attributes: layout.attributes.as_slice(),
  }
}

pub fn compute_shader_builder() -> ShaderComputePipelineBuilder {
  ShaderComputePipelineBuilder::new(&|stage| Box::new(ShaderAPINagaImpl::new(stage)))
}

pub trait ComputeIntoPipelineExt {
  fn create_compute_pipeline(
    self,
    device: impl AsRef<GPUDevice>,
  ) -> Result<GPUComputePipeline, ShaderBuildError>;
}

impl ComputeIntoPipelineExt for ShaderComputePipelineBuilder {
  fn create_compute_pipeline(
    self,
    device: impl AsRef<GPUDevice>,
  ) -> Result<GPUComputePipeline, ShaderBuildError> {
    let log_result = self.log_result;
    let result = self.build()?;

    let device = device.as_ref();

    let (entry, shader) = result.shader;

    let naga_compute = shader.downcast::<naga::Module>().unwrap();
    let module = device.create_shader_module_by_shader_api(*naga_compute, log_result);

    let (raw_layouts, layouts, pipeline_layout) = create_layouts(device, &result.bindings);

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      module: &module,
      entry_point: Some(&entry),
      compilation_options: Default::default(),
      cache: None,
    });

    Ok(GPUPipeline::new(pipeline, raw_layouts, layouts))
  }
}
