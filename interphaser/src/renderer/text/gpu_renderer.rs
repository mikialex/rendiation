use std::borrow::Cow;
use std::mem;

use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::TextQuadInstance;

use super::WebGPUxTextPrimitive;

pub struct TextWebGPURenderer {
  transform: UniformBufferData<[f32; 16]>,
  sampler: wgpu::Sampler,
  bindgroup_layout: wgpu::BindGroupLayout,
  bindgroup: wgpu::BindGroup,
  raw: wgpu::RenderPipeline,
}

pub struct TextureWriteData<'a> {
  pub data: &'a [u8],
  pub size: Size,
}

impl<'a> WebGPUTexture2dSource for TextureWriteData<'a> {
  fn format(&self) -> wgpu::TextureFormat {
    wgpu::TextureFormat::R8Unorm
  }

  fn as_bytes(&self) -> &[u8] {
    self.data
  }

  fn size(&self) -> Size {
    self.size
  }

  fn bytes_per_pixel(&self) -> usize {
    1
  }
}

impl TextWebGPURenderer {
  pub fn new(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
    view_size: Vec2<f32>,
    cache_view: &wgpu::TextureView,
  ) -> Self {
    let transform =
      UniformBufferData::create(device, orthographic_projection(view_size.x, view_size.y));

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: filter_mode,
      min_filter: filter_mode,
      mipmap_filter: filter_mode,
      ..Default::default()
    });

    let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("wgpu_glyph::TextGPURenderer uniforms"),
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(mem::size_of::<[f32; 16]>() as u64),
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Sampler {
            filtering: true,
            comparison: false,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
      ],
    });

    let bindgroup = create_bindgroup(device, &uniform_layout, &transform, &sampler, cache_view);

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      push_constant_ranges: &[],
      bind_group_layouts: &[&uniform_layout],
    });

    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: Some("Glyph Shader"),
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./glyph.wgsl"))),
    });

    let raw = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: "vs_main",
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: mem::size_of::<TextQuadInstance>() as u64,
          step_mode: wgpu::VertexStepMode::Instance,
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
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: &[wgpu::ColorTargetState {
          format: render_format,
          blend: Some(wgpu::BlendState::ALPHA_BLENDING),
          write_mask: wgpu::ColorWrites::ALL,
        }],
      }),
    });

    Self {
      sampler,
      transform,
      bindgroup_layout: uniform_layout,
      bindgroup,
      raw,
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &wgpu::Queue) {
    *self.transform = orthographic_projection(size.x, size.y);
    self.transform.update(queue);
  }

  pub fn draw<'r>(&'r self, render_pass: &mut GPURenderPass<'r>, text: &'r WebGPUxTextPrimitive) {
    render_pass.set_pipeline(&self.raw);
    render_pass.set_bind_group(0, &self.bindgroup, &[]);
    render_pass.set_vertex_buffer(0, text.vertex_buffer.slice(..));

    render_pass.draw(0..4, 0..text.length);
  }

  pub fn cache_resized(&mut self, device: &wgpu::Device, cache_view: &wgpu::TextureView) {
    self.bindgroup = create_bindgroup(
      device,
      &self.bindgroup_layout,
      &self.transform,
      &self.sampler,
      cache_view,
    );
  }
}

/// Helper function to generate a generate a transform matrix.
pub fn orthographic_projection(width: f32, height: f32) -> [f32; 16] {
  #[rustfmt::skip]
    [
        2.0 / width , 0.0, 0.0, 0.0,
        0.0, -2.0 / height , 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, 1.0, 0.0, 1.0,
    ]
}

fn create_bindgroup(
  device: &wgpu::Device,
  layout: &wgpu::BindGroupLayout,
  transform: &UniformBufferData<[f32; 16]>,
  sampler: &wgpu::Sampler,
  cache: &wgpu::TextureView,
) -> wgpu::BindGroup {
  device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("wgpu_glyph::TextGPURenderer uniforms"),
    layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: transform.as_bindable(),
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
