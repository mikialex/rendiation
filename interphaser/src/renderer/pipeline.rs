use rendiation_webgpu::VertexBufferSourceType;

use crate::{renderer::UIGlobalParameter, UIVertex};

pub fn create_solid_pipeline(
  device: &wgpu::Device,
  target_format: wgpu::TextureFormat,
  global_uniform_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("ui_solid_pipeline_layout"),
    bind_group_layouts: &[&global_uniform_bind_group_layout],
    push_constant_ranges: &[],
  });

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
      
      ",
    vertex_header = UIVertex::get_shader_header(),
    global_header = UIGlobalParameter::get_shader_header(),
  );

  let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source.as_str())),
  });

  let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("ui_solid_pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      entry_point: "vs_main",
      module: &shader,
      buffers: &[UIVertex::vertex_layout()],
    },
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleList,
      clamp_depth: false,
      conservative: false,
      cull_mode: None,
      front_face: wgpu::FrontFace::default(),
      polygon_mode: wgpu::PolygonMode::default(),
      strip_index_format: None,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState {
      alpha_to_coverage_enabled: false,
      count: 1,
      mask: !0,
    },

    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format: target_format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
      }],
    }),
  });

  render_pipeline
}

pub struct TextureBindGroup {
  pub bindgroup: wgpu::BindGroup,
}

impl TextureBindGroup {
  pub fn new(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
  ) -> Self {
    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(view),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(sampler),
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

  pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            multisampled: false,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Sampler {
            comparison: false,
            filtering: true,
          },
          count: None,
        },
      ],
    })
  }
}

pub fn create_texture_pipeline(
  device: &wgpu::Device,
  target_format: wgpu::TextureFormat,
  global_uniform_bind_group_layout: &wgpu::BindGroupLayout,
  texture_bg_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("ui_tex_pipeline_layout"),
    bind_group_layouts: &[&global_uniform_bind_group_layout, texture_bg_layout],
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
          return  textureSample(r_color, r_sampler, in.uv) * in.color;
      }}
      
      ",
    vertex_header = UIVertex::get_shader_header(),
    global_header = UIGlobalParameter::get_shader_header(),
    texture_group = TextureBindGroup::get_shader_header()
  );

  let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source.as_str())),
  });

  let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("ui_solid_pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      entry_point: "vs_main",
      module: &shader,
      buffers: &[UIVertex::vertex_layout()],
    },
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleList,
      clamp_depth: false,
      conservative: false,
      cull_mode: None,
      front_face: wgpu::FrontFace::default(),
      polygon_mode: wgpu::PolygonMode::default(),
      strip_index_format: None,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState {
      alpha_to_coverage_enabled: false,
      count: 1,
      mask: !0,
    },

    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format: target_format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
      }],
    }),
  });

  render_pipeline
}
