use crate::test_renderer::TestRenderer;
use crate::application::*;
use crate::renderer::r#const::OPENGL_TO_WGPU_MATRIX;
use crate::renderer::*;
use crate::util::*;
use crate::vertex::*;
use rendiation_math::*;
use rendiation_render_entity::{Camera, PerspectiveCamera};
use rendiation::*;


pub struct Rinecraft {
  vertex_buf: WGPUBuffer,
  index_buf: WGPUBuffer,
  index_count: usize,
  bind_group: WGPUBindGroup,
  camera: PerspectiveCamera,
  uniform_buf: WGPUBuffer,
  pipeline: WGPUPipeline,
}

impl Rinecraft {
  fn generate_matrix(&mut self, aspect_ratio: f32) -> Mat4<f32> {
    self.camera.aspect = aspect_ratio;
    self.camera.update_projection();
    let mx_projection = *self.camera.get_projection_matrix();

    let mx_view = Mat4::lookat_rh(
      Vec3::new(5f32, 5.0, 5.0),
      Vec3::new(0f32, 0.0, 0.0),
      Vec3::unit_y(),
    );

    let mx_correction = OPENGL_TO_WGPU_MATRIX;
    mx_correction * mx_projection * mx_view
  }
}

impl Application<TestRenderer> for Rinecraft {
  fn init(renderer: &WGPURenderer<TestRenderer>) -> (Self, Option<wgpu::CommandBuffer>) {
    let device = &renderer.device;
    let sc_desc = &renderer.swap_chain_descriptor;
    // code
    use crate::renderer::*;
    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();

    pipeline_builder
      .vertex_shader(include_str!("./shader.vert"))
      .frag_shader(include_str!("./shader.frag"))
      .binding_group(
        BindGroupLayoutBuilder::new()
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
          }),
      );

    let pipeline = pipeline_builder.build::<Vertex>(device, sc_desc);

    //

    // Create the vertex and index buffers
    let (vertex_data, index_data) = create_vertices();
    let vertex_buf = WGPUBuffer::new(device, &vertex_data, wgpu::BufferUsage::VERTEX);
    let index_buf = WGPUBuffer::new(device, &index_data, wgpu::BufferUsage::INDEX);

    let mut init_encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

    // Create the texture
    let size = 512u32;
    let img = create_texels(size as usize);
    let texture = WGPUTexture::new(device, &mut init_encoder, &img);
    let texture_view = texture.make_default_view();

    // Create other resources
    let sampler = WGPUSampler::new(device);

    let mut camera = PerspectiveCamera::new();
    camera.aspect = sc_desc.width as f32 / sc_desc.height as f32;
    camera.update_projection();
    let mx_projection = *camera.get_projection_matrix();
    let mx_view = Mat4::lookat_rh(
      Vec3::new(5f32, 5.0, 5.0),
      Vec3::new(0f32, 0.0, 0.0),
      Vec3::unit_y(),
    );
    let mx_correction = OPENGL_TO_WGPU_MATRIX;
    let mx_total = mx_correction * mx_projection * mx_view;

    // let mx_total = self.generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    let uniform_buf = WGPUBuffer::new(
      device,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    // Create bind group
    let bind_group = BindGroupBuilder::new()
      .buffer(&uniform_buf)
      .texture(&texture_view)
      .sampler(&sampler)
      .build(device, &pipeline.bind_group_layouts[0]);

    // Done
    let this = Rinecraft {
      vertex_buf,
      index_buf,
      camera,
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

  fn resize(&mut self, renderer: &WGPURenderer<TestRenderer>) -> Option<wgpu::CommandBuffer> {
    let device = &renderer.device;
    let sc_desc = &renderer.swap_chain_descriptor;

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

    let mx_total = self.generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    self.uniform_buf.update(device, &mut encoder, mx_ref);

    Some(encoder.finish())
  }

  fn render(
    &mut self,
    frame: &wgpu::SwapChainOutput,
    device: &wgpu::Device,
    renderer: &mut TestRenderer,
  ) -> wgpu::CommandBuffer {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

    {
      let mut pass = WGPURenderPass::build()
        .output_with_clear(&frame.view, (0.1, 0.2, 0.3, 1.0))
        .with_depth(&renderer.depth.get_view())
        .create(&mut encoder);

      let rpass = &mut pass.gpu_pass;
      rpass.set_pipeline(&self.pipeline.pipeline);
      rpass.set_bind_group(0, &self.bind_group.gpu_bindgroup, &[]);
      rpass.set_index_buffer(&self.index_buf.get_gpu_buffer(), 0);
      rpass.set_vertex_buffers(0, &[(&self.vertex_buf.get_gpu_buffer(), 0)]);
      rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);
    }

    encoder.finish()
  }
}

// trait WGPURenderabled{
//   fn render(device: &wgpu::Device, encoder: wgpu::CommandEncoder);
// }
