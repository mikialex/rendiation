use std::borrow::Cow;
use std::mem;

use rendiation_algebra::*;
use rendiation_texture::Size;
use webgpu::*;

use crate::TextQuadInstance;

use super::WebGPUxTextPrimitive;

pub struct TextWebGPURenderer {
  transform: UniformBufferData<Mat4<f32>>,
  sampler: webgpu::Sampler,
  bindgroup_layout: webgpu::BindGroupLayout,
  bindgroup: webgpu::BindGroup,
  raw: webgpu::RenderPipeline,
}

#[derive(Debug)]
pub struct TextureWriteData<'a> {
  pub data: &'a [u8],
  pub size: Size,
}

impl<'a> WebGPUTexture2dSource for TextureWriteData<'a> {
  fn format(&self) -> webgpu::TextureFormat {
    webgpu::TextureFormat::R8Unorm
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
    device: &webgpu::GPUDevice,
    filter_mode: webgpu::FilterMode,
    render_format: webgpu::TextureFormat,
    view_size: Vec2<f32>,
    cache_view: &webgpu::TextureView,
  ) -> Self {
    let transform =
      UniformBufferData::create(device, orthographic_projection(view_size.x, view_size.y));

    let sampler = device.create_sampler(&webgpu::SamplerDescriptor {
      address_mode_u: webgpu::AddressMode::ClampToEdge,
      address_mode_v: webgpu::AddressMode::ClampToEdge,
      address_mode_w: webgpu::AddressMode::ClampToEdge,
      mag_filter: filter_mode,
      min_filter: filter_mode,
      mipmap_filter: filter_mode,
      ..Default::default()
    });

    let uniform_layout = device.create_bind_group_layout(&webgpu::BindGroupLayoutDescriptor {
      label: Some("TextWebGPURenderer uniforms"),
      entries: &[
        webgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: webgpu::ShaderStages::VERTEX,
          ty: webgpu::BindingType::Buffer {
            ty: webgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: webgpu::BufferSize::new(mem::size_of::<[f32; 16]>() as u64),
          },
          count: None,
        },
        webgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: webgpu::ShaderStages::FRAGMENT,
          ty: webgpu::BindingType::Sampler(webgpu::SamplerBindingType::Filtering),
          count: None,
        },
        webgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: webgpu::ShaderStages::FRAGMENT,
          ty: webgpu::BindingType::Texture {
            sample_type: webgpu::TextureSampleType::Float { filterable: true },
            view_dimension: webgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
      ],
    });

    let bindgroup = create_bindgroup(device, &uniform_layout, &transform, &sampler, cache_view);

    let layout = device.create_pipeline_layout(&webgpu::PipelineLayoutDescriptor {
      label: None,
      push_constant_ranges: &[],
      bind_group_layouts: &[&uniform_layout],
    });

    let shader = device.create_shader_module(webgpu::ShaderModuleDescriptor {
      label: Some("Glyph Shader"),
      source: webgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./glyph.wgsl"))),
    });

    let raw = device.create_render_pipeline(&webgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&layout),
      vertex: webgpu::VertexState {
        module: &shader,
        entry_point: "vs_main",
        buffers: &[webgpu::VertexBufferLayout {
          array_stride: mem::size_of::<TextQuadInstance>() as u64,
          step_mode: webgpu::VertexStepMode::Instance,
          attributes: &webgpu::vertex_attr_array![
              0 => Float32x3,
              1 => Float32x2,
              2 => Float32x2,
              3 => Float32x2,
              4 => Float32x4,
          ],
        }],
      },
      primitive: webgpu::PrimitiveState {
        topology: webgpu::PrimitiveTopology::TriangleStrip,
        front_face: webgpu::FrontFace::Cw,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: webgpu::MultisampleState::default(),
      fragment: Some(webgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: &[Some(webgpu::ColorTargetState {
          format: render_format,
          blend: Some(webgpu::BlendState::ALPHA_BLENDING),
          write_mask: webgpu::ColorWrites::ALL,
        })],
      }),
      multiview: None,
    });

    Self {
      sampler,
      transform,
      bindgroup_layout: uniform_layout,
      bindgroup,
      raw,
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &webgpu::Queue) {
    self
      .transform
      .mutate(|t| *t = orthographic_projection(size.x, size.y));
    self.transform.update(queue);
  }

  pub fn draw<'r>(&'r self, render_pass: &mut GPURenderPass<'r>, text: &'r WebGPUxTextPrimitive) {
    render_pass.set_pipeline(&self.raw);
    render_pass.set_bind_group(0, &self.bindgroup, &[]);
    render_pass.set_vertex_buffer(0, text.vertex_buffer.slice(..));

    render_pass.draw(0..4, 0..text.length);
  }

  pub fn cache_resized(&mut self, device: &webgpu::Device, cache_view: &webgpu::TextureView) {
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
pub fn orthographic_projection(width: f32, height: f32) -> Mat4<f32> {
  #[rustfmt::skip]
    [
      2.0 / width, 0.0,          0.0, 0.0,
      0.0,        -2.0 / height, 0.0, 0.0,
      0.0,         0.0,          1.0, 0.0,
     -1.0,         1.0,          0.0, 1.0,
    ].into()
}

fn create_bindgroup(
  device: &webgpu::Device,
  layout: &webgpu::BindGroupLayout,
  transform: &UniformBufferData<Mat4<f32>>,
  sampler: &webgpu::Sampler,
  cache: &webgpu::TextureView,
) -> webgpu::BindGroup {
  device.create_bind_group(&webgpu::BindGroupDescriptor {
    label: Some("TextWebGPURenderer uniforms"),
    layout,
    entries: &[
      webgpu::BindGroupEntry {
        binding: 0,
        resource: transform.as_bindable(),
      },
      webgpu::BindGroupEntry {
        binding: 1,
        resource: webgpu::BindingResource::Sampler(sampler),
      },
      webgpu::BindGroupEntry {
        binding: 2,
        resource: webgpu::BindingResource::TextureView(cache),
      },
    ],
  })
}
