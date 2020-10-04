use crate::{WebGLProgram, WebGLRenderer, WebGLTexture, WebGLVertexBuffer};

use rendiation_ral::*;
use std::ops::Range;
use web_sys::*;

impl RALBackend for WebGLRenderer {
  type RenderTarget = Option<WebGlFramebuffer>;
  type RenderPass = WebGLRenderer;
  type Renderer = WebGLRenderer;
  type ShaderBuildSource = (); // todo
  type Shading = WebGLProgram;
  type BindGroup = ();
  type IndexBuffer = WebGlBuffer;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = (); // we use uniform value now
  type Texture = WebGLTexture;
  type Sampler = ();

  fn create_shading(renderer: &mut WebGLRenderer, des: &Self::ShaderBuildSource) -> Self::Shading {
    WebGLProgram::new(renderer, des)
  }
  fn dispose_shading(renderer: &mut WebGLRenderer, shading: Self::Shading) {
    renderer.gl.delete_program(Some(shading.program()))
  }

  fn apply_shading(pass: &mut Self::RenderPass, shading: &Self::Shading) {
    pass.use_program(shading)
  }
  fn apply_bindgroup(_pass: &mut Self::RenderPass, _index: usize, _bindgroup: &Self::BindGroup) {}

  fn create_uniform_buffer(_renderer: &mut WebGLRenderer, _data: &[u8]) -> Self::UniformBuffer {
    // renderer.create_uniform_buffer(data)
    todo!()
  }
  fn dispose_uniform_buffer(_renderer: &mut Self::Renderer, _uniform: Self::UniformBuffer) {
    // renderer.delete_uniform_buffer(uniform)
    todo!()
  }
  fn update_uniform_buffer(
    _renderer: &mut Self::Renderer,
    _gpu: &mut Self::UniformBuffer,
    _data: &[u8],
    _range: Range<usize>, // todo
  ) {
    todo!()
  }

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    renderer.create_index_buffer(data)
  }

  fn dispose_index_buffer(renderer: &mut Self::Renderer, buffer: Self::IndexBuffer) {
    renderer.dispose_index_buffer(buffer)
  }

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    layout: RALVertexBufferDescriptor,
  ) -> Self::VertexBuffer {
    renderer.create_vertex_buffer(data, layout)
  }
  fn dispose_vertex_buffer(renderer: &mut Self::Renderer, buffer: Self::VertexBuffer) {
    renderer.dispose_vertex_buffer(buffer)
  }

  fn render_object(
    object: &RenderObject<Self>,
    pass: &mut Self::RenderPass,
    resources: &ResourceManager<Self>,
  ) {
    // shading bind
    pass.texture_slot_states.reset_slots();
    let shading_storage = resources.shadings.get_shading_boxed(object.shading);
    shading_storage.apply(pass, resources);

    let program = shading_storage.get_gpu();
    let program = resources.shading_gpu.get(program).unwrap();
    program.upload(pass, resources, shading_storage.shading_provider_as_any());

    // geometry bind
    let geometry = &resources.get_geometry(object.geometry).resource();

    pass.attribute_states.prepare_new_bindings();
    geometry.index_buffer.map(|b| {
      let index = resources.get_index_buffer(b);
      pass.set_index_buffer(Some(index.resource().as_ref()));
    });
    geometry
      .vertex_buffers
      .iter()
      .enumerate()
      .for_each(|(i, &v)| {
        let buffer = resources.get_vertex_buffer(v).resource();
        pass.set_vertex_buffer(i as i32, buffer);
      });
    pass.disable_old_unused_bindings();

    // let range = &geometry.draw_range;
    // renderer.gl.draw_elements_with_i32(
    //   WebGl2RenderingContext::TRIANGLES,
    //   range.start as i32,
    //   WebGl2RenderingContext::UNSIGNED_INT,
    //   range.end as i32,
    // );
  }
}
