use crate::application::*;
use crate::renderer::r#const::OPENGL_TO_WGPU_MATRIX;
use crate::renderer::*;
mod util;
use util::*;

pub struct Rinecraft {
  vertex_buf: WGPUBuffer,
  index_buf: WGPUBuffer,
  index_count: usize,
  bind_group: wgpu::BindGroup,
  uniform_buf: WGPUBuffer,
  pipeline: WGPUPipeline,
}

impl Rinecraft {
  fn generate_matrix(aspect_ratio: f32) -> cgmath::Matrix4<f32> {
    let mx_projection = cgmath::perspective(cgmath::Deg(45f32), aspect_ratio, 1.0, 10.0);
    let mx_view = cgmath::Matrix4::look_at(
      cgmath::Point3::new(1.5f32, -5.0, 3.0),
      cgmath::Point3::new(0f32, 0.0, 0.0),
      cgmath::Vector3::unit_z(),
    );
    let mx_correction = OPENGL_TO_WGPU_MATRIX;
    mx_correction * mx_projection * mx_view
  }
}

impl Application for Rinecraft {
  fn init(
    sc_desc: &wgpu::SwapChainDescriptor,
    device: &wgpu::Device,
  ) -> (Self, Option<wgpu::CommandBuffer>) {
    // code
    use crate::renderer::*;
    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();

    pipeline_builder
      .vertex_shader(
        r#"
            #version 450

            layout(location = 0) in vec4 a_Pos;
            layout(location = 1) in vec2 a_TexCoord;
            layout(location = 0) out vec2 v_TexCoord;

            layout(set = 0, binding = 0) uniform Locals {
                mat4 u_Transform;
            };

            void main() {
                v_TexCoord = a_TexCoord;
                gl_Position = u_Transform * a_Pos;
            }
        "#,
      )
      .frag_shader(
        r#"
          #version 450

          layout(location = 0) in vec2 v_TexCoord;
          layout(location = 0) out vec4 o_Target;
          layout(set = 0, binding = 1) uniform texture2D t_Color;
          layout(set = 0, binding = 2) uniform sampler s_Color;
          
          void main() {
              vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);
              float mag = length(v_TexCoord-vec2(0.5));
              o_Target = mix(tex, vec4(0.0), mag*mag);
          }
      "#,
      )
      .binding(wgpu::BindGroupLayoutBinding {
        binding: 0,
        visibility: wgpu::ShaderStage::VERTEX,
        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
      })
      .binding(wgpu::BindGroupLayoutBinding {
        binding: 1,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::SampledTexture {
          multisampled: false,
          dimension: wgpu::TextureViewDimension::D2,
        },
      })
      .binding(wgpu::BindGroupLayoutBinding {
        binding: 2,
        visibility: wgpu::ShaderStage::FRAGMENT,
        ty: wgpu::BindingType::Sampler,
      });

    let pipeline = pipeline_builder.build::<Vertex>(device, sc_desc);

    //

    // Create the vertex and index buffers
    let (vertex_data, index_data) = create_vertices();
    let vertex_buf = WGPUBuffer::new(device, &vertex_data, wgpu::BufferUsage::VERTEX);
    let index_buf = WGPUBuffer::new(device, &index_data, wgpu::BufferUsage::INDEX);

    // Create the texture
    let size = 512u32;
    let texels = create_texels(size as usize);
    let texture_extent = wgpu::Extent3d {
      width: size,
      height: size,
      depth: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      size: texture_extent,
      array_layer_count: 1,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });
    let texture_view = texture.create_default_view();

    let temp_buf = device
      .create_buffer_mapped(texels.len(), wgpu::BufferUsage::COPY_SRC)
      .fill_from_slice(&texels);

    let mut init_encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    init_encoder.copy_buffer_to_texture(
      wgpu::BufferCopyView {
        buffer: &temp_buf,
        offset: 0,
        row_pitch: 4 * size,
        image_height: size,
      },
      wgpu::TextureCopyView {
        texture: &texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d {
          x: 0.0,
          y: 0.0,
          z: 0.0,
        },
      },
      texture_extent,
    );

    // Create other resources
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: -100.0,
      lod_max_clamp: 100.0,
      compare_function: wgpu::CompareFunction::Always,
    });
    let mx_total = Self::generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    let uniform_buf = WGPUBuffer::new(device, mx_ref, wgpu::BufferUsage::UNIFORM |  wgpu::BufferUsage::COPY_DST);

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &pipeline.bind_groups_layouts[0], // todo
      bindings: &[
        wgpu::Binding {
          binding: 0,
          resource: wgpu::BindingResource::Buffer {
            buffer: &uniform_buf.get_gpu_buffer(),
            range: 0..64,
          },
        },
        wgpu::Binding {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&texture_view),
        },
        wgpu::Binding {
          binding: 2,
          resource: wgpu::BindingResource::Sampler(&sampler),
        },
      ],
    });

    // Done
    let this = Rinecraft {
      vertex_buf,
      index_buf,
      index_count: index_data.len(),
      bind_group,
      uniform_buf,
      pipeline,
    };
    (this, Some(init_encoder.finish()))
  }

  fn update(&mut self, _event: winit::event::WindowEvent) {
    //empty
  }

  fn resize(
    &mut self,
    sc_desc: &wgpu::SwapChainDescriptor,
    device: &wgpu::Device,
  ) -> Option<wgpu::CommandBuffer> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

    let mx_total = Self::generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    self.uniform_buf.update(device, &mut encoder, mx_ref);

    Some(encoder.finish())
  }

  fn render(
    &mut self,
    frame: &wgpu::SwapChainOutput,
    device: &wgpu::Device,
  ) -> wgpu::CommandBuffer {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    {
      let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: &frame.view,
          resolve_target: None,
          load_op: wgpu::LoadOp::Clear,
          store_op: wgpu::StoreOp::Store,
          clear_color: wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
          },
        }],
        depth_stencil_attachment: None,
      });
      rpass.set_pipeline(&self.pipeline.pipeline);
      rpass.set_bind_group(0, &self.bind_group, &[]);
      rpass.set_index_buffer(&self.index_buf.get_gpu_buffer(), 0);
      rpass.set_vertex_buffers(0, &[(&self.vertex_buf.get_gpu_buffer(), 0)]);
      rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);
    }

    encoder.finish()
  }
}
