use core::num::NonZeroU64;
use std::borrow::Cow;
use std::mem;

use crate::ui::renderer::text::text_quad_instance::Instance;

use super::cache::Cache;

pub struct Pipeline {
  transform: wgpu::Buffer,
  current_transform: [f32; 16],
  sampler: wgpu::Sampler,
  cache: Cache,
  uniform_layout: wgpu::BindGroupLayout,
  uniforms: wgpu::BindGroup,
  raw: wgpu::RenderPipeline,
}

impl Pipeline {
  pub fn new(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
    cache_width: u32,
    cache_height: u32,
  ) -> Pipeline {
    build(
      device,
      filter_mode,
      render_format,
      None,
      cache_width,
      cache_height,
    )
  }

  pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
    render_pass.set_pipeline(&self.raw);
    render_pass.set_bind_group(0, &self.uniforms, &[]);
    render_pass.set_vertex_buffer(0, self.instances.slice(..));

    render_pass.draw(0..4, 0..self.current_instances as u32);
  }
}

impl Pipeline {
  pub fn update_cache(
    &mut self,
    device: &wgpu::Device,
    staging_belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    offset: [u16; 2],
    size: [u16; 2],
    data: &[u8],
  ) {
    self
      .cache
      .update(device, staging_belt, encoder, offset, size, data);
  }

  pub fn increase_cache_size(&mut self, device: &wgpu::Device, width: u32, height: u32) {
    self.cache = Cache::new(device, width, height);

    self.uniforms = create_uniforms(
      device,
      &self.uniform_layout,
      &self.transform,
      &self.sampler,
      &self.cache.view,
    );
  }

  pub fn upload(
    &mut self,
    device: &wgpu::Device,
    staging_belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    instances: &[Instance],
  ) {
    if instances.is_empty() {
      self.current_instances = 0;
      return;
    }

    if instances.len() > self.supported_instances {
      self.instances = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("wgpu_glyph::Pipeline instances"),
        size: mem::size_of::<Instance>() as u64 * instances.len() as u64,
        usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
      });

      self.supported_instances = instances.len();
    }

    let instances_bytes = bytemuck::cast_slice(instances);

    if let Some(size) = NonZeroU64::new(instances_bytes.len() as u64) {
      let mut instances_view = staging_belt.write_buffer(encoder, &self.instances, 0, size, device);

      instances_view.copy_from_slice(instances_bytes);
    }

    self.current_instances = instances.len();
  }
}

// Helpers
#[cfg_attr(rustfmt, rustfmt_skip)]
const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

fn build(
  device: &wgpu::Device,
  filter_mode: wgpu::FilterMode,
  render_format: wgpu::TextureFormat,
  depth_stencil: Option<wgpu::DepthStencilState>,
  cache_width: u32,
  cache_height: u32,
) -> Pipeline {
  use wgpu::util::DeviceExt;

  let transform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: None,
    contents: bytemuck::cast_slice(&IDENTITY_MATRIX),
    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
  });

  let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    address_mode_u: wgpu::AddressMode::ClampToEdge,
    address_mode_v: wgpu::AddressMode::ClampToEdge,
    address_mode_w: wgpu::AddressMode::ClampToEdge,
    mag_filter: filter_mode,
    min_filter: filter_mode,
    mipmap_filter: filter_mode,
    ..Default::default()
  });

  let cache = Cache::new(device, cache_width, cache_height);

  let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("wgpu_glyph::Pipeline uniforms"),
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStage::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: wgpu::BufferSize::new(mem::size_of::<[f32; 16]>() as u64),
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::Sampler {
          filtering: true,
          comparison: false,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: false },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      },
    ],
  });

  let uniforms = create_uniforms(device, &uniform_layout, &transform, &sampler, &cache.view);

  let instances = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("wgpu_glyph::Pipeline instances"),
    size: mem::size_of::<Instance>() as u64 * Instance::INITIAL_AMOUNT as u64,
    usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
    mapped_at_creation: false,
  });

  let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    push_constant_ranges: &[],
    bind_group_layouts: &[&uniform_layout],
  });

  let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: Some("Glyph Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./glyph.wgsl"))),
    flags: wgpu::ShaderFlags::all(),
  });

  let raw = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: Some(&layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: "vs_main",
      buffers: &[wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Instance>() as u64,
        step_mode: wgpu::InputStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x2,
            4 => Float32x4,
        ],
      }],
    },
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleStrip,
      front_face: wgpu::FrontFace::Cw,
      ..Default::default()
    },
    depth_stencil,
    multisample: wgpu::MultisampleState::default(),
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format: render_format,
        blend: Some(wgpu::BlendState {
          color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
          },
          alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
          },
        }),
        write_mask: wgpu::ColorWrite::ALL,
      }],
    }),
  });

  Pipeline {
    transform,
    sampler,
    cache,
    uniform_layout,
    uniforms,
    raw,
    current_transform: [0.0; 16],
  }
}

fn create_uniforms(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  transform: &wgpu::Buffer,
  sampler: &wgpu::Sampler,
  cache: &wgpu::TextureView,
) -> wgpu::BindGroup {
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("wgpu_glyph::Pipeline uniforms"),
    layout: layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
          buffer: transform,
          offset: 0,
          size: None,
        }),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::Sampler(sampler),
      },
      wgpu::BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::TextureView(cache),
      },
    ],
  })
}
