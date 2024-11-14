use std::sync::Arc;

use rendiation_shader_api::*;
use rendiation_webgpu::*;

use crate::*;

pub struct WebGPUCanvasRenderer {
  gpu: GPU,
  pool: AttachmentPool,
}

impl TriangulationBasedRendererImpl for WebGPUCanvasRenderer {
  type Image = RenderTargetView;

  fn render(&mut self, target: &Self::Image, content: &GraphicsRepresentation) {
    let device = &self.gpu.device;

    let metadata_buffer = create_gpu_readonly_storage(content.object_meta.as_slice(), device);
    let vertex_buffer = bytemuck::cast_slice(&content.vertices);
    let vertex_buffer = create_gpu_buffer(vertex_buffer, BufferUsages::VERTEX, device);
    let index_buffer = bytemuck::cast_slice(&content.indices);
    let index_buffer = create_gpu_buffer(index_buffer, BufferUsages::INDEX, device);

    // encode.

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool);

    let _ = pass("gui")
      .with_color(target.clone(), load())
      .render_ctx(&mut ctx);

    ctx.final_submit();
  }
}

struct UIContextGPU {
  texture_atlas: GPUTextureView,
  font_atlas: GPUTextureView,
}

struct UIPrimitivesGPU {
  vertex_buffer: GPUBufferResourceView,
  index_buffer: GPUBufferResourceView,
  metadata_buffer: StorageBufferReadOnlyDataView<[ObjectMetaData]>,
}

impl ShaderPassBuilder for UIPrimitivesGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.metadata_buffer);
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(&self.index_buffer, IndexFormat::Uint32);
    ctx.set_vertex_buffer_by_buffer_resource_view_next(&self.vertex_buffer);
  }
}

impl GraphicsShaderProvider for UIPrimitivesGPU {
  fn build(&self, builder: &mut rendiation_shader_api::ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      builder.register_vertex::<GraphicsVertex>(VertexStepMode::Vertex);
      builder.primitive_state.topology = PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;

      let position = builder.query::<GeometryPosition2D>();
      let color = builder.query::<GeometryColorWithAlphaPremultiplied>();
      let uv = builder.query::<GeometryUV>();

      let meta_index = builder.query::<UIMetadata>();

      let metadata = binding.bind_by(&self.metadata_buffer);
      let metadata = metadata.index(meta_index).load().expand();

      let position = metadata.world_transform * position;
      let image_id = metadata.image_id;
      let uv = uv * metadata.uv_scale + metadata.uv_offset;
    })

    // builder.fragment(|builder, binding| {
    //   let uv = builder.query::<FragmentUv>()?;
    //   let texture = binding.binding::<GPU2DTextureView>();
    //   let sampler = binding.binding::<GPUSamplerView>();

    //   builder.store_fragment_out(0, texture.sample(sampler, uv))
    // })
  }
}
