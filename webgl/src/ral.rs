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
    let gl = &renderer.gl;
    let buffer = renderer
      .gl
      .create_buffer()
      .ok_or("failed to create ubo buffer")
      .unwrap();
    gl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, Some(&buffer));
    gl.buffer_data_with_u8_array_and_src_offset(
      WebGl2RenderingContext::UNIFORM_BUFFER,
      data,
      WebGl2RenderingContext::STATIC_DRAW,
      0,
    );
    return buffer;
  }
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer) {
    renderer.gl.delete_buffer(Some(&uniform));
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

  fn render_object(
    _object: &RenderObject<Self>,
    _pass: &mut Self::RenderPass,
    _resources: &ResourceManager<Self>,
  ) {
    todo!()

    // let resources = &scene.resources;
    // let shading = resources.get_shading(self.shading_index).resource();
    // let geometry = &resources.get_geometry(self.geometry_index).resource();
    // let program = shading.gpu();

    // renderer.use_program(program.program());

    // // geometry bind
    // renderer.attribute_states.prepare_new_bindings();
    // geometry.index_buffer.map(|b| {
    //   let index = resources.get_index_buffer(b);
    //   renderer.set_index_buffer(index.resource().as_ref());
    // });
    // geometry.vertex_buffers.iter().for_each(|v| {
    //   let buffer = resources.get_vertex_buffer(v.1).resource();
    //   let att_location = program.query_attribute_location(v.0);
    //   renderer.set_vertex_buffer(att_location, buffer);
    // });
    // renderer
    //   .attribute_states
    //   .disable_old_unused_bindings(&renderer.gl);

    // // shading bind
    // renderer.texture_slot_states.reset_slots();
    // for i in 0..shading.get_parameters_count() {
    //   let parameter_group = resources
    //     .get_shading_param_group(shading.get_parameter(i))
    //     .resource();
    //   parameter_group.items().iter().for_each(|p| {
    //     use ShadingParameterType::*;
    //     match &p.1 {
    //       UniformBuffer(_index) => {
    //         // let _uniform = resources.get_uniform(index).resource();
    //         todo!()
    //       }
    //       UniformValue(_index) => {
    //         // let uniform_value = resources.get_uniform_value(index).resource();
    //         // program.upload_uniform_value(uniform_value, p.0, &renderer.gl);
    //       }
    //       SampledTexture(_) => {
    //         // let texture = resources.get_sampled_texture(index).respirce();
    //       }
    //       _ => panic!("unsupported webgl resource type"),
    //     }
    //   })
    // }

    // let range = &geometry.draw_range;
    // renderer.gl.draw_elements_with_i32(
    //   WebGl2RenderingContext::TRIANGLES,
    //   range.start as i32,
    //   WebGl2RenderingContext::UNSIGNED_INT,
    //   range.end as i32,
    // );
  }
}
