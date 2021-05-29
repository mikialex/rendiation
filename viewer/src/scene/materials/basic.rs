use std::borrow::Cow;

use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;

use crate::{renderer::Renderer, scene::OriginForward};

use super::{
  MaterialCPUResource, MaterialGPUResource, PipelineResourceManager, SceneMaterialRenderPrepareCtx,
};

pub struct BasicMaterial {
  pub color: Vec3<f32>,
}

pub struct BasicMaterialGPU {
  uniform: wgpu::Buffer,
  bindgroup_layout: wgpu::BindGroupLayout,
  bindgroup: wgpu::BindGroup,
}

impl MaterialGPUResource<OriginForward> for BasicMaterialGPU {
  type Source = BasicMaterial;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<OriginForward>,
  ) {
    //
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
    style: &OriginForward,
  ) {
  }
}

impl MaterialCPUResource for BasicMaterial {
  type GPU = BasicMaterialGPU;

  fn create<S>(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> Self::GPU {
    let bind_group_layout =
      renderer
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          label: None,
          entries: &[
            wgpu::BindGroupLayoutEntry {
              binding: 0,
              visibility: wgpu::ShaderStage::VERTEX,
              ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(64),
              },
              count: None,
            },
            wgpu::BindGroupLayoutEntry {
              binding: 1,
              visibility: wgpu::ShaderStage::FRAGMENT,
              ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
              },
              count: None,
            },
            wgpu::BindGroupLayoutEntry {
              binding: 2,
              visibility: wgpu::ShaderStage::FRAGMENT,
              ty: wgpu::BindingType::Sampler {
                comparison: false,
                filtering: true,
              },
              count: None,
            },
          ],
        });

    use wgpu::util::DeviceExt;
    let uniform_buf: wgpu::Buffer = todo!();
    // renderer
    //   .device
    //   .create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //     label: Some("Uniform Buffer"),
    //     contents: bytemuck::cast_slice(todo!()),
    //     usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    //   });
    let texture_view = todo!();

    // Create other resources
    let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::FilterMode::Nearest,
      ..Default::default()
    });

    let bind_group = renderer
      .device
      .create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(&texture_view),
          },
          wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(&sampler),
          },
        ],
        label: None,
      });

    let vertex_size = std::mem::size_of::<Vertex>();
    let vertex_buffers = [wgpu::VertexBufferLayout {
      array_stride: vertex_size as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x4,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x2,
          offset: 4 * 4,
          shader_location: 1,
        },
      ],
    }];

    let shader = renderer
      .device
      .create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
          r#"
      [[location(0)]]
      var<in> in_position: vec4<f32>;
      [[location(1)]]
      var<in> in_tex_coord_vs: vec2<f32>;
      [[location(0)]]
      var<out> out_tex_coord: vec2<f32>;

      [[builtin(position)]]
      var<out> out_position: vec4<f32>;
      
      [[block]]
      struct Locals {
          transform: mat4x4<f32>;
      };
      [[group(0), binding(0)]]
      var r_locals: Locals;
      
      [[stage(vertex)]]
      fn vs_main() {
          out_tex_coord = in_tex_coord_vs;
          out_position = r_locals.transform * in_position;
      }
      
      [[location(0)]]
      var<in> in_tex_coord_fs: vec2<f32>;
      [[location(0)]]
      var<out> out_color: vec4<f32>;
      [[group(0), binding(1)]]
      var r_color: texture_2d<f32>;
      [[group(0), binding(2)]]
      var r_sampler: sampler;
      
      [[stage(fragment)]]
      fn fs_main() {
          var tex: vec4<f32> = textureSample(r_color, r_sampler, in_tex_coord_fs);
          out_color = tex;
          //TODO: support `length` and `mix` functions
          //var mag: f32 = length(in_tex_coord_fs-vec2<f32>(0.5, 0.5));
          //out_color = vec4<f32>(mix(tex.xyz, vec3<f32>(0.0, 0.0, 0.0), mag*mag), 1.0);
      }
      
      [[stage(fragment)]]
      fn fs_wire() {
          out_color = vec4<f32>(0.0, 0.5, 0.0, 0.5);
      }
      
      "#,
        )),
        flags: renderer.create_shader_flags(),
      });

    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
      });

    let pipeline = renderer
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader,
          entry_point: "vs_main",
          buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader,
          entry_point: "fs_main",
          targets: &[renderer.get_prefer_target_format().into()],
        }),
        primitive: wgpu::PrimitiveState {
          cull_mode: wgpu::Face::Back.into(),
          ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
      });

    // BasicMaterialGPU {
    //   uniform,
    //   bindgroup_layout,
    //   bindgroup,
    //   pipeline,
    // }
  }
  //
}
