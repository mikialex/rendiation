use crate::{make_webgl_program, CALBackend, SceneShadingDescriptor, SceneUniform, WebGLRenderer};
use web_sys::*;

struct WebGLCALBackend {}

impl CALBackend for WebGLCALBackend {
  type Renderer = WebGLRenderer;
  type Shading = WebGlProgram;
  fn create_shading(renderer: &mut WebGLRenderer, des: &SceneShadingDescriptor) -> Self::Shading {
    make_webgl_program(&renderer.gl, &des.vertex_shader_str, &des.frag_shader_str).unwrap()
  }
  fn dispose_shading(_renderer: &mut WebGLRenderer, _shading: Self::Shading) {
    todo!()
  }
  type Uniform = WebGlBuffer;
  fn create_uniform_buffer(_renderer: &mut WebGLRenderer, _des: SceneUniform) -> Self::Uniform {
    todo!()
  }

  type IndexBuffer = WebGlBuffer;
  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    let buffer = renderer
      .gl
      .create_buffer()
      .ok_or("failed to create buffer")
      .unwrap();
    renderer
      .gl
      .bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&buffer));
    unsafe {
      // unsafe for transmute and avoid allocation(cause heap grow and move in wasm)
      let transmuted = std::mem::transmute::<&[u8], &[u16]>(data);
      let vert_array = js_sys::Uint16Array::view(transmuted);
      renderer.gl.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        &vert_array,
        WebGlRenderingContext::STATIC_DRAW,
      );
    };
    buffer
  }

  type VertexBuffer = WebGlBuffer;
  fn create_vertex_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::VertexBuffer {
    // todo!()
    let buffer = renderer
      .gl
      .create_buffer()
      .ok_or("failed to create buffer")
      .unwrap();
    renderer
      .gl
      .bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));
    unsafe {
      let transmuted = std::mem::transmute::<&[u8], &[f32]>(data);
      let vert_array = js_sys::Float32Array::view(transmuted);
      renderer.gl.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ARRAY_BUFFER,
        &vert_array,
        WebGlRenderingContext::STATIC_DRAW,
      );
    };
    buffer
  }
}
