use std::borrow::Cow;
use std::mem;

use wgpu::util::DeviceExt;

use crate::ui::renderer::text::text_quad_instance::Instance;

use super::cache::Cache;
use super::GPUxUITextPrimitive;

pub struct TextRendererPipeline {
  transform: wgpu::Buffer,
  current_transform: [f32; 16],
  sampler: wgpu::Sampler,
  cache: Cache,
  bindgroup_layout: wgpu::BindGroupLayout,
  bindgroup: wgpu::BindGroup,
  raw: wgpu::RenderPipeline,
}

impl TextRendererPipeline {
  pub fn new(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
    cache_width: u32,
    cache_height: u32,
  ) -> Self {
    build(
      device,
      filter_mode,
      render_format,
      None,
      cache_width,
      cache_height,
    )
  }

  pub fn draw<'r>(&'r self, render_pass: &mut wgpu::RenderPass<'r>, text: &'r GPUxUITextPrimitive) {
    render_pass.set_pipeline(&self.raw);
    render_pass.set_bind_group(0, &self.bindgroup, &[]);
    render_pass.set_vertex_buffer(0, text.vertex_buffer.slice(..));

    render_pass.draw(0..4, 0..text.length);
  }
}

impl TextRendererPipeline {
  pub fn update_cache(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    offset: [u16; 2],
    size: [u16; 2],
    data: &[u8],
  ) {
    self.cache.update(device, encoder, offset, size, data);
  }

  pub fn increase_cache_size(&mut self, device: &wgpu::Device, width: u32, height: u32) {
    self.cache = Cache::new(device, width, height);

    self.bindgroup = create_bindgroup(
      device,
      &self.bindgroup_layout,
      &self.transform,
      &self.sampler,
      &self.cache.view,
    );
  }

  pub fn create_gpu_text(
    &mut self,
    device: &wgpu::Device,
    instances: &[Instance],
  ) -> Option<GPUxUITextPrimitive> {
    if instances.is_empty() {
      return None;
    }
    let instances_bytes = bytemuck::cast_slice(instances);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: instances_bytes,
      usage: wgpu::BufferUsage::VERTEX,
    });

    GPUxUITextPrimitive {
      vertex_buffer,
      length: instances.len() as u32,
    }
    .into()
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

/// Helper function to generate a generate a transform matrix.
pub fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
  #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, -2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, 1.0, 0.0, 1.0,
    ]
}

fn build(
  device: &wgpu::Device,
  filter_mode: wgpu::FilterMode,
  render_format: wgpu::TextureFormat,
  depth_stencil: Option<wgpu::DepthStencilState>,
  cache_width: u32,
  cache_height: u32,
) -> TextRendererPipeline {
  let buffer = orthographic_projection(1000, 1000);
  let transform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: None,
    contents: bytemuck::cast_slice(&buffer),
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
    label: Some("wgpu_glyph::TextRendererPipeline uniforms"),
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

  let bindgroup = create_bindgroup(device, &uniform_layout, &transform, &sampler, &cache.view);

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

  TextRendererPipeline {
    transform,
    sampler,
    cache,
    bindgroup_layout: uniform_layout,
    bindgroup,
    raw,
    current_transform: [0.0; 16],
  }
}

fn create_bindgroup(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  transform: &wgpu::Buffer,
  sampler: &wgpu::Sampler,
  cache: &wgpu::TextureView,
) -> wgpu::BindGroup {
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("wgpu_glyph::TextRendererPipeline uniforms"),
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
