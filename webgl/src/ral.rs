use crate::{UniformValue, WebGLProgram, WebGLRenderer, WebGLTexture, WebGLVertexBuffer};

use rendiation_ral::*;
use web_sys::*;

impl RALBackend for WebGLRenderer {
  type RenderTarget = Option<WebGlFramebuffer>;
  type Renderer = WebGLRenderer;
  type Shading = WebGLProgram;
  type ShadingParameterGroup = ();
  type IndexBuffer = Option<WebGlBuffer>;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = WebGlBuffer;
  type UniformValue = UniformValue;
  type Texture = ();
  type Sampler = ();
  type SampledTexture = WebGLTexture;

  fn create_shading(renderer: &mut WebGLRenderer, des: &SceneShadingDescriptor) -> Self::Shading {
    // extra shader conversion should do in sal
    WebGLProgram::new(renderer, des)
  }
  fn dispose_shading(renderer: &mut WebGLRenderer, shading: Self::Shading) {
    renderer.gl.delete_program(Some(shading.program()))
  }

  fn create_uniform_buffer(renderer: &mut WebGLRenderer, des: SceneUniform) -> Self::UniformBuffer {
    let gl = &renderer.gl;
    let buffer = renderer
      .gl
      .create_buffer()
      .ok_or("failed to create ubo buffer")
      .unwrap();
    gl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, Some(&buffer));
    gl.buffer_data_with_u8_array_and_src_offset(
      WebGl2RenderingContext::UNIFORM_BUFFER,
      des.value.as_byte(),
      WebGl2RenderingContext::STATIC_DRAW,
      0,
    );
    return buffer;
  }
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer) {
    renderer.gl.delete_buffer(Some(&uniform));
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
