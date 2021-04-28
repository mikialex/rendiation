use rendiation_algebra::Vec3;

use crate::*;

pub struct BasicMaterial {
  pub color: Vec3<f32>,
}

pub struct BasicMaterialGPU {
  uniform: wgpu::Buffer,
  bindgroup_layout: wgpu::BindGroupLayout,
  bindgroup: wgpu::BindGroup,
  pipeline: wgpu::RenderPipeline,
}

impl MaterialGPUResource for BasicMaterialGPU {
  type Source = BasicMaterial;
  fn update(&mut self, source: &Self::Source, renderer: &mut Renderer) {
    //
  }
}

impl MaterialCPUResource for BasicMaterial {
  type GPU = BasicMaterialGPU;

  fn create(&mut self, renderer: &mut Renderer) -> Self::GPU {
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

    let uniform = todo!();

    // let bind_group = renderer
    //   .device
    //   .create_bind_group(&wgpu::BindGroupDescriptor {
    //     layout: &bind_group_layout,
    //     entries: &[
    //       wgpu::BindGroupEntry {
    //         binding: 0,
    //         resource: uniform_buf.as_entire_binding(),
    //       },
    //       wgpu::BindGroupEntry {
    //         binding: 1,
    //         resource: wgpu::BindingResource::TextureView(&texture_view),
    //       },
    //       wgpu::BindGroupEntry {
    //         binding: 2,
    //         resource: wgpu::BindingResource::Sampler(&sampler),
    //       },
    //     ],
    //     label: None,
    //   });

    // let vertex_buffers = [wgpu::VertexBufferLayout {
    //   array_stride: vertex_size as wgpu::BufferAddress,
    //   step_mode: wgpu::InputStepMode::Vertex,
    //   attributes: &[
    //     wgpu::VertexAttribute {
    //       format: wgpu::VertexFormat::Float4,
    //       offset: 0,
    //       shader_location: 0,
    //     },
    //     wgpu::VertexAttribute {
    //       format: wgpu::VertexFormat::Float2,
    //       offset: 4 * 4,
    //       shader_location: 1,
    //     },
    //   ],
    // }];

    // let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    //   label: None,
    //   source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    //   flags,
    // });

    // let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    //   label: None,
    //   layout: Some(&pipeline_layout),
    //   vertex: wgpu::VertexState {
    //     module: &shader,
    //     entry_point: "vs_main",
    //     buffers: &vertex_buffers,
    //   },
    //   fragment: Some(wgpu::FragmentState {
    //     module: &shader,
    //     entry_point: "fs_main",
    //     targets: &[sc_desc.format.into()],
    //   }),
    //   primitive: wgpu::PrimitiveState {
    //     cull_mode: wgpu::CullMode::Back,
    //     ..Default::default()
    //   },
    //   depth_stencil: None,
    //   multisample: wgpu::MultisampleState::default(),
    // });

    // BasicMaterialGPU {
    //   uniform,
    //   bindgroup_layout,
    //   bindgroup,
    //   pipeline,
    // }
  }
  //
}
