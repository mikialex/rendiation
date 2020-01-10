use crate::application::*;
use crate::geometry::*;
use crate::renderer::r#const::OPENGL_TO_WGPU_MATRIX;
use crate::renderer::*;
use crate::test_renderer::TestRenderer;
use crate::util::*;
use rendiation::*;
use rendiation_math::*;
use rendiation_render_entity::{Camera, PerspectiveCamera};

pub struct Rinecraft {
  camera: PerspectiveCamera,
  bind_group: WGPUBindGroup,
  uniform_buf: WGPUBuffer,
  cube: StandardGeometry,
  pipeline: WGPUPipeline,
}

impl Application<TestRenderer> for Rinecraft {
  fn init(renderer: &mut WGPURenderer<TestRenderer>) -> Self {
    let device = &renderer.device;
    let sc_desc = &renderer.swap_chain_descriptor;
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

    let pipeline = pipeline_builder.build::<StandardGeometry>(device, sc_desc);

    //

    // Create the vertex and index buffers
    let (vertex_data, index_data) = create_vertices();
    let cube = StandardGeometry::new(vertex_data, index_data, &renderer);

    // Create the texture
    let size = 512u32;
    let img = create_texels(size as usize);
    let texture = WGPUTexture::new(device, &mut renderer.encoder, &img);
    let texture_view = texture.make_default_view();

    // Create other resources
    let sampler = WGPUSampler::new(device);

    let mut camera = PerspectiveCamera::new();
    camera.resize((sc_desc.width as f32, sc_desc.height as f32));
    camera.update_projection();
    camera.transform.matrix = Mat4::lookat_rh(
      Vec3::new(5f32, 5.0, 5.0),
      Vec3::new(0f32, 0.0, 0.0),
      Vec3::unit_y(),
    );
    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();

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
    Rinecraft {
      cube,
      camera,
      bind_group,
      uniform_buf,
      pipeline,
    }
  }

  fn update(
    &mut self,
    _event: winit::event::WindowEvent,
    renderer: &mut WGPURenderer<TestRenderer>,
  ) {
    //empty
    // self.camera.transform.position += Vec3::new(0.0, 0.0, 0.1);
    // self.camera.transform.update_matrix_by_compose();
    // let mx_total = OPENGL_TO_WGPU_MATRIX * self.camera.get_vp_matrix();
    // let mx_ref: &[f32; 16] = mx_total.as_ref();
    // self
    //   .uniform_buf
    //   .update(&renderer.device, &mut renderer.encoder, mx_ref);
  }

  fn resize(&mut self, renderer: &mut WGPURenderer<TestRenderer>) {
    let sc_desc = &renderer.swap_chain_descriptor;

    self
      .camera
      .resize((sc_desc.width as f32, sc_desc.height as f32));
    let mx_total = OPENGL_TO_WGPU_MATRIX * self.camera.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    self
      .uniform_buf
      .update(&renderer.device, &mut renderer.encoder, mx_ref);
  }

  fn render(
    &mut self,
    frame: &wgpu::TextureView,
    device: &wgpu::Device,
    renderer: &mut TestRenderer,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    let mut pass = WGPURenderPass::build()
      .output_with_clear(frame, (0.1, 0.2, 0.3, 1.0))
      .with_depth(&renderer.depth.get_view())
      .create(encoder);
    {
      let rpass = &mut pass.gpu_pass;
      rpass.set_pipeline(&self.pipeline.pipeline);
      rpass.set_bind_group(0, &self.bind_group.gpu_bindgroup, &[]);
    }
    self.cube.provide_gpu(&mut pass);
    {
      let rpass = &mut pass.gpu_pass;
      rpass.draw_indexed(0..self.cube.get_full_count(), 0, 0..1);
    }
  }
}

// trait WGPURenderabled{
//   fn render(device: &wgpu::Device, encoder: wgpu::CommandEncoder);
// }
