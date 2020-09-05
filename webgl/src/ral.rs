use crate::{UniformValue, WebGLProgram, WebGLRenderer, WebGLTexture, WebGLVertexBuffer};

use rendiation_ral::*;
use std::ops::Range;
use web_sys::*;

impl RALBackend for WebGLRenderer {
  type RenderTarget = Option<WebGlFramebuffer>;
  type RenderPass = WebGLRenderer;
  type Renderer = WebGLRenderer;
  type Shading = WebGLProgram;
  type BindGroup = ();
  type IndexBuffer = Option<WebGlBuffer>;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = WebGlBuffer;
  type UniformValue = UniformValue;
  type Texture = ();
  type TextureView = WebGLTexture;
  type Sampler = ();

  fn create_shading(renderer: &mut WebGLRenderer, des: &SceneShadingDescriptor) -> Self::Shading {
    // extra shader conversion should do in sal
    WebGLProgram::new(renderer, des)
  }
  fn dispose_shading(renderer: &mut WebGLRenderer, shading: Self::Shading) {
    renderer.gl.delete_program(Some(shading.program()))
  }

  fn create_uniform_buffer(renderer: &mut WebGLRenderer, data: &[u8]) -> Self::UniformBuffer {
    renderer.create_uniform_buffer(data)
  }
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer) {
    renderer.delete_uniform_buffer(uniform)
  }
  // fn update_uniform_buffer(_renderer: &mut Self::Renderer, _data: &[u8], _range: Range<usize>){
  //   todo!()
  // }
  fn update_uniform_buffer(
    _renderer: &mut Self::Renderer,
    _gpu: &mut Self::UniformBuffer,
    _data: &[u8],
    _range: Range<usize>, // todo
  ) {
    todo!()
    // gpu.update(renderer, data);
  }

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    let buffer = renderer
      .gl
      .create_buffer()
      .ok_or("failed to create buffer")
      .unwrap();
    renderer
      .gl
      .bind_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, Some(&buffer));
    unsafe {
      // unsafe for transmute and avoid allocation(cause heap grow and move in wasm)
      let transmuted = std::mem::transmute::<&[u8], &[u16]>(data);
      let vert_array = js_sys::Uint16Array::view(transmuted);
      renderer.gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
        &vert_array,
        WebGl2RenderingContext::STATIC_DRAW,
      );
    };
    Some(buffer)
  }

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    layout: RALVertexBufferDescriptor,
  ) -> Self::VertexBuffer {
    let buffer = renderer
      .gl
      .create_buffer()
      .ok_or("failed to create buffer")
      .unwrap();
    renderer
      .gl
      .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));
    unsafe {
      let transmuted = std::mem::transmute::<&[u8], &[f32]>(data);
      let vert_array = js_sys::Float32Array::view(transmuted);
      renderer.gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &vert_array,
        WebGl2RenderingContext::STATIC_DRAW,
      );
    };
    WebGLVertexBuffer { buffer, layout }
  }
}
