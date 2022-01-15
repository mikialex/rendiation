use webgpu::VertexBufferSourceType;

use crate::{renderer::UIGlobalParameter, UIVertex};

pub fn create_solid_pipeline(
  device: &webgpu::Device,
  target_format: webgpu::TextureFormat,
  global_uniform_bind_group_layout: &webgpu::BindGroupLayout,
) -> webgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&webgpu::PipelineLayoutDescriptor {
    label: Some("ui_solid_pipeline_layout"),
    bind_group_layouts: &[global_uniform_bind_group_layout],
    push_constant_ranges: &[],
  });

  let vertex_header = UIVertex::get_shader_header();
  let global_header = UIGlobalParameter::get_shader_header();

  let shader_source = format!(
    "
      {global_header}

      struct VertexOutput {{
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] color: vec4<f32>;
      }};

      [[stage(vertex)]]
      fn vs_main(
        {vertex_header}
      ) -> VertexOutput {{
        var out: VertexOutput;

        out.color = color;

        out.position = vec4<f32>(
            2.0 * position.x / ui_global_parameter.screen_size.x - 1.0,
            1.0 - 2.0 * position.y / ui_global_parameter.screen_size.y,
            0.0,
            1.0,
        );

        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return in.color;
      }}
      "
  );

  let shader = device.create_shader_module(&webgpu::ShaderModuleDescriptor {
    label: None,
    source: webgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source.as_str())),
  });

  let render_pipeline = device.create_render_pipeline(&webgpu::RenderPipelineDescriptor {
    label: Some("ui_solid_pipeline"),
    layout: Some(&pipeline_layout),
    vertex: webgpu::VertexState {
      entry_point: "vs_main",
      module: &shader,
      buffers: &[UIVertex::vertex_layout().as_raw()],
    },
    primitive: webgpu::PrimitiveState {
      topology: webgpu::PrimitiveTopology::TriangleList,
      unclipped_depth: false,
      conservative: false,
      cull_mode: None,
      front_face: webgpu::FrontFace::default(),
      polygon_mode: webgpu::PolygonMode::default(),
      strip_index_format: None,
    },
    depth_stencil: None,
    multisample: webgpu::MultisampleState {
      alpha_to_coverage_enabled: false,
      count: 1,
      mask: !0,
    },

    fragment: Some(webgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[webgpu::ColorTargetState {
        format: target_format,
        blend: Some(webgpu::BlendState::ALPHA_BLENDING),
        write_mask: webgpu::ColorWrites::ALL,
      }],
    }),
    multiview: None,
  });

  render_pipeline
}

pub struct TextureBindGroup {
  pub bindgroup: webgpu::BindGroup,
}

impl TextureBindGroup {
  pub fn new(
    device: &webgpu::Device,
    layout: &webgpu::BindGroupLayout,
    sampler: &webgpu::Sampler,
    view: &webgpu::TextureView,
  ) -> Self {
    let bindgroup = device.create_bind_group(&webgpu::BindGroupDescriptor {
      layout,
      entries: &[
        webgpu::BindGroupEntry {
          binding: 0,
          resource: webgpu::BindingResource::TextureView(view),
        },
        webgpu::BindGroupEntry {
          binding: 1,
          resource: webgpu::BindingResource::Sampler(sampler),
        },
      ],
      label: None,
    });
    Self { bindgroup }
  }
}

impl TextureBindGroup {
  fn get_shader_header() -> &'static str {
    "
    [[group(1), binding(0)]]
    var r_color: texture_2d<f32>;

    [[group(1), binding(1)]]
    var r_sampler: sampler;
    "
  }

  pub fn create_bind_group_layout(device: &webgpu::Device) -> webgpu::BindGroupLayout {
    device.create_bind_group_layout(&webgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        webgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: webgpu::ShaderStages::FRAGMENT,
          ty: webgpu::BindingType::Texture {
            multisampled: false,
            sample_type: webgpu::TextureSampleType::Float { filterable: true },
            view_dimension: webgpu::TextureViewDimension::D2,
          },
          count: None,
        },
        webgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: webgpu::ShaderStages::FRAGMENT,
          ty: webgpu::BindingType::Sampler(webgpu::SamplerBindingType::Filtering),
          count: None,
        },
      ],
    })
  }
}

pub fn create_texture_pipeline(
  device: &webgpu::Device,
  target_format: webgpu::TextureFormat,
  global_uniform_bind_group_layout: &webgpu::BindGroupLayout,
  texture_bg_layout: &webgpu::BindGroupLayout,
) -> webgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&webgpu::PipelineLayoutDescriptor {
    label: Some("ui_tex_pipeline_layout"),
    bind_group_layouts: &[global_uniform_bind_group_layout, texture_bg_layout],
    push_constant_ranges: &[],
  });

  let shader_source = format!(
    "
      {global_header}
      {texture_group}

      struct VertexOutput {{
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] color: vec4<f32>;
        [[location(1)]] uv: vec2<f32>;
      }};

      [[stage(vertex)]]
      fn vs_main(
        {vertex_header}
      ) -> VertexOutput {{
        var out: VertexOutput;

        out.color = color;
        out.uv = uv;

        out.position = vec4<f32>(
            2.0 * position.x / ui_global_parameter.screen_size.x - 1.0,
            1.0 - 2.0 * position.y / ui_global_parameter.screen_size.y,
            0.0,
            1.0,
        );

        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return textureSample(r_color, r_sampler, in.uv) * in.color;
      }}
      
      ",
    vertex_header = UIVertex::get_shader_header(),
    global_header = UIGlobalParameter::get_shader_header(),
    texture_group = TextureBindGroup::get_shader_header()
  );

  let shader = device.create_shader_module(&webgpu::ShaderModuleDescriptor {
    label: None,
    source: webgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source.as_str())),
  });

  let render_pipeline = device.create_render_pipeline(&webgpu::RenderPipelineDescriptor {
    label: Some("ui_solid_pipeline"),
    layout: Some(&pipeline_layout),
    vertex: webgpu::VertexState {
      entry_point: "vs_main",
      module: &shader,
      buffers: &[UIVertex::vertex_layout().as_raw()],
    },
    primitive: webgpu::PrimitiveState {
      topology: webgpu::PrimitiveTopology::TriangleList,
      conservative: false,
      cull_mode: None,
      front_face: webgpu::FrontFace::default(),
      polygon_mode: webgpu::PolygonMode::default(),
      strip_index_format: None,
      unclipped_depth: false,
    },
    depth_stencil: None,
    multisample: webgpu::MultisampleState {
      alpha_to_coverage_enabled: false,
      count: 1,
      mask: !0,
    },

    fragment: Some(webgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[webgpu::ColorTargetState {
        format: target_format,
        blend: Some(webgpu::BlendState::ALPHA_BLENDING),
        write_mask: webgpu::ColorWrites::ALL,
      }],
    }),
    multiview: None,
  });

  render_pipeline
}
