use crate::{
  WebGLProgram, WebGLProgramBuildSource, WebGLRenderer, WebGLTexture, WebGLVertexBuffer,
};

use rendiation_ral::*;
use std::ops::Range;
use web_sys::*;

pub struct WebGL;

impl RAL for WebGL {
  type RenderTarget = Option<WebGlFramebuffer>;
  type RenderPass = WebGLRenderer;
  type Renderer = WebGLRenderer;
  type ShaderBuildSource = WebGLProgramBuildSource; // todo
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
  fn apply_bindgroup(_pass: &mut Self::RenderPass, _index: usize, _bindgroup: &Self::BindGroup) {
    // empty impl
  }

  fn apply_vertex_buffer(pass: &mut Self::RenderPass, index: i32, vertex: &Self::VertexBuffer) {
    pass.set_vertex_buffer(index, vertex);
  }
  fn apply_index_buffer(pass: &mut Self::RenderPass, index: &Self::IndexBuffer) {
    pass.set_index_buffer(Some(index));
  }

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

  fn set_viewport(pass: &mut Self::RenderPass, viewport: &Viewport) {
    // todo check if has depth info and log
    pass.gl.viewport(
      viewport.x as i32,
      viewport.y as i32,
      viewport.w as i32,
      viewport.h as i32,
    );
  }

  fn draw_indexed(pass: &mut Self::RenderPass, topology: PrimitiveTopology, range: Range<u32>) {
    pass.gl.draw_elements_with_i32(
      ral_topology_to_webgl_topology(topology),
      (range.end - range.start) as i32,
      WebGl2RenderingContext::UNSIGNED_INT,
      range.end as i32,
    );
  }
  fn draw_none_indexed(
    pass: &mut Self::RenderPass,
    topology: PrimitiveTopology,
    range: Range<u32>,
  ) {
    pass.gl.draw_arrays(
      ral_topology_to_webgl_topology(topology),
      range.start as i32,
      (range.end - range.start) as i32,
    );
  }

  fn render_drawcall(
    drawcall: &Drawcall<Self>,
    pass: &mut Self::RenderPass,
    resources: &ResourceManager<Self>,
  ) {
    // shading bind
    pass.texture_slot_states.reset_slots();

    let shading_storage = resources.shadings.get_shading_boxed(drawcall.shading);
    shading_storage.apply(pass, resources);

    let program = shading_storage.get_gpu();
    let program = resources.shading_gpu.get(program).unwrap();
    program.upload(pass, resources, shading_storage.shading_provider_as_any());

    // geometry bind
    pass.attribute_states.prepare_new_bindings();

    let geometry = resources.get_geometry(drawcall.geometry);
    geometry.apply(pass, resources);

    pass.disable_old_unused_bindings();

    geometry.draw(pass);
  }
}

fn ral_topology_to_webgl_topology(t: PrimitiveTopology) -> u32 {
  use PrimitiveTopology::*;
  match t {
    TriangleList => WebGl2RenderingContext::TRIANGLES,
    _ => panic!("not support"),
  }
}
