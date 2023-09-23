use std::borrow::Cow;
use std::mem;

use rendiation_algebra::*;
use rendiation_texture::Size;

use super::WebGPUxTextPrimitive;
use crate::TextQuadInstance;
use crate::*;

pub struct TextWebGPURenderer {
  transform: UniformBufferDataView<Mat4<f32>>,
  sampler: Sampler,
  bindgroup_layout: BindGroupLayout,
  bindgroup: BindGroup,
  raw: RenderPipeline,
}

#[derive(Debug)]
pub struct TextureWriteData<'a> {
  pub data: &'a [u8],
  pub size: Size,
}

impl<'a> WebGPU2DTextureSource for TextureWriteData<'a> {
  fn format(&self) -> TextureFormat {
    TextureFormat::R8Unorm
  }

  fn as_bytes(&self) -> &[u8] {
    self.data
  }

  fn size(&self) -> Size {
    self.size
  }
}

impl TextWebGPURenderer {
  pub fn new(
    device: &GPUDevice,
    filter_mode: FilterMode,
    render_format: TextureFormat,
    view_size: Vec2<f32>,
    cache_view: &TextureView,
  ) -> Self {
    let transform =
      UniformBufferDataView::create(device, orthographic_projection(view_size.x, view_size.y));

    let sampler = device.create_sampler(&SamplerDescriptor {
      address_mode_u: AddressMode::ClampToEdge,
      address_mode_v: AddressMode::ClampToEdge,
      address_mode_w: AddressMode::ClampToEdge,
      mag_filter: filter_mode,
      min_filter: filter_mode,
      mipmap_filter: filter_mode,
      ..Default::default()
    });

    let uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: Some("TextWebGPURenderer uniforms"),
      entries: &[
        BindGroupLayoutEntry {
          binding: 0,
          visibility: rendiation_webgpu::ShaderStages::VERTEX_FRAGMENT,
          ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<[f32; 16]>() as u64),
          },
          count: None,
        },
        BindGroupLayoutEntry {
          binding: 1,
          visibility: rendiation_webgpu::ShaderStages::VERTEX_FRAGMENT,
          ty: BindingType::Sampler(SamplerBindingType::Filtering),
          count: None,
        },
        BindGroupLayoutEntry {
          binding: 2,
          visibility: rendiation_webgpu::ShaderStages::VERTEX_FRAGMENT,
          ty: BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
      ],
    });

    let bindgroup = create_bindgroup(device, &uniform_layout, &transform, &sampler, cache_view);

    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: None,
      push_constant_ranges: &[],
      bind_group_layouts: &[&uniform_layout],
    });

    let shader = device.create_shader_module(ShaderModuleDescriptor {
      label: Some("Glyph Shader"),
      source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("./glyph.wgsl"))),
    });

    let raw = device.create_render_pipeline(&RenderPipelineDescriptor {
      label: None,
      layout: Some(&layout),
      vertex: VertexState {
        module: &shader,
        entry_point: "vs_main",
        buffers: &[VertexBufferLayout {
          array_stride: mem::size_of::<TextQuadInstance>() as u64,
          step_mode: VertexStepMode::Instance,
          attributes: &vertex_attr_array![
              0 => Float32x3,
              1 => Float32x2,
              2 => Float32x2,
              3 => Float32x2,
              4 => Float32x4,
          ],
        }],
      },
      primitive: PrimitiveState {
        topology: PrimitiveTopology::TriangleStrip,
        front_face: FrontFace::Cw,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: MultisampleState::default(),
      fragment: Some(FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: &[Some(ColorTargetState {
          format: render_format,
          blend: Some(BlendState::ALPHA_BLENDING),
          write_mask: ColorWrites::ALL,
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

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &Queue) {
    self
      .transform
      .mutate(|t| *t = orthographic_projection(size.x, size.y));
    self.transform.upload(queue);
  }

  pub fn draw<'r>(&'r self, render_pass: &mut GPURenderPass<'r>, text: &'r WebGPUxTextPrimitive) {
    render_pass.set_pipeline(&self.raw);
    render_pass.set_bind_group(0, &self.bindgroup, &[]);
    render_pass.set_vertex_buffer(0, text.vertex_buffer.slice(..));

    render_pass.draw(0..4, 0..text.length);
  }

  pub fn cache_resized(&mut self, device: &Device, cache_view: &TextureView) {
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
  device: &Device,
  layout: &BindGroupLayout,
  transform: &UniformBufferDataView<Mat4<f32>>,
  sampler: &Sampler,
  cache: &TextureView,
) -> BindGroup {
  device.create_bind_group(&BindGroupDescriptor {
    label: Some("TextWebGPURenderer uniforms"),
    layout,
    entries: &[
      BindGroupEntry {
        binding: 0,
        resource: transform.as_bindable(),
      },
      BindGroupEntry {
        binding: 1,
        resource: BindingResource::Sampler(sampler),
      },
      BindGroupEntry {
        binding: 2,
        resource: BindingResource::TextureView(cache),
      },
    ],
  })
}
