use crate::application::*;
use crate::geometry::*;
use crate::renderer::r#const::OPENGL_TO_WGPU_MATRIX;
use crate::renderer::*;
use crate::util::*;
use crate::watch::*;
use rendiation::*;
use rendiation_math::*;
use rendiation_render_entity::{Camera, PerspectiveCamera};

impl GPUItem<PerspectiveCamera> for WGPUBuffer {
  fn create_gpu(item: &PerspectiveCamera, renderer: &mut WGPURenderer) -> Self {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();

    WGPUBuffer::new(
      &renderer.device,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    )
  }
  fn update_gpu(&mut self, item: &PerspectiveCamera, renderer: &mut WGPURenderer) {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();

    self.update(&renderer.device, &mut renderer.encoder, mx_ref);
  }
}

pub struct Rinecraft {
  camera: GPUPair<PerspectiveCamera, WGPUBuffer>,
  bind_group: WGPUBindGroup,
  cube: StandardGeometry,
  pipeline: WGPUPipeline,
  depth: WGPUAttachmentTexture,
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer) -> Self {
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

    let pipeline =
      pipeline_builder.build::<StandardGeometry>(&renderer.device, &renderer.swap_chain_descriptor);

    // Create the vertex and index buffers
    let (vertex_data, index_data) = create_vertices();
    let cube = StandardGeometry::new(vertex_data, index_data, &renderer);

    // Create the texture
    let size = 512u32;
    let img = create_texels(size as usize);
    let texture = WGPUTexture::new(&renderer.device, &mut renderer.encoder, &img);
    let texture_view = texture.make_default_view();

    // Create other resources
    let sampler = WGPUSampler::new(&renderer.device);

    let mut camera = GPUPair::new(PerspectiveCamera::new(), renderer);
    let sc_desc = &renderer.swap_chain_descriptor;
    camera.resize((sc_desc.width as f32, sc_desc.height as f32));
    camera.update_projection();
    camera.transform.matrix = Mat4::lookat_rh(
      Vec3::new(5f32, 5.0, 5.0),
      Vec3::new(0f32, 0.0, 0.0),
      Vec3::unit_y(),
    );

    // Create bind group
    let bind_group = BindGroupBuilder::new()
      .buffer(camera.get_update_gpu(renderer))
      .texture(&texture_view)
      .sampler(&sampler)
      .build(&renderer.device, &pipeline.bind_group_layouts[0]);

    let depth = WGPUAttachmentTexture::new_as_depth(
      &renderer.device,
      wgpu::TextureFormat::Depth32Float,
      renderer.size,
    );

    // Done
    Rinecraft {
      cube,
      camera,
      bind_group,
      pipeline,
      depth,
    }
  }

  fn update(&mut self, _event: winit::event::WindowEvent, renderer: &mut WGPURenderer) {
    //empty
    // self.camera.transform.position += Vec3::new(0.0, 0.0, 0.1);
    // self.camera.transform.update_matrix_by_compose();
    // let mx_total = OPENGL_TO_WGPU_MATRIX * self.camera.get_vp_matrix();
    // let mx_ref: &[f32; 16] = mx_total.as_ref();
    // self
    //   .uniform_buf
    //   .update(&renderer.device, &mut renderer.encoder, mx_ref);
  }

  fn resize(&mut self, renderer: &mut WGPURenderer) {
    let sc_desc = &renderer.swap_chain_descriptor;

    self.depth.resize(&renderer.device, renderer.size);
    self
      .camera
      .resize((sc_desc.width as f32, sc_desc.height as f32));
  }

  fn render(&mut self, renderer: &mut WGPURenderer) {
    
    self.camera.get_update_gpu(renderer);

    let frame = &renderer.swap_chain.get_next_texture().view;
    let mut pass = WGPURenderPass::build()
      .output_with_clear(frame, (0.1, 0.2, 0.3, 1.0))
      .with_depth(&self.depth.get_view())
      .create(&mut renderer.encoder);
    {
      let rpass = &mut pass.gpu_pass;
      rpass.set_pipeline(&self.pipeline.pipeline);
      rpass.set_bind_group(0, &self.bind_group.gpu_bindgroup, &[]);
    }
    self.cube.render(&mut pass);
  }
}
