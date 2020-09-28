use crate::{WebGLProgram, WebGLRenderer, WebGLTexture, WebGLVertexBuffer};

use rendiation_ral::*;
use std::ops::Range;
use web_sys::*;

impl RALBackend for WebGLRenderer {
  type RenderTarget = Option<WebGlFramebuffer>;
  type RenderPass = WebGLRenderer;
  type Renderer = WebGLRenderer;
  type ShaderBuildSource = SceneShadingDescriptor; // todo
  type Shading = WebGLProgram;
  type BindGroup = ();
  type IndexBuffer = WebGlBuffer;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = WebGlBuffer;
  type Texture = ();
  type TextureView = WebGLTexture;
  type Sampler = ();

  fn create_shading(renderer: &mut WebGLRenderer, des: &Self::ShaderBuildSource) -> Self::Shading {
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
    resources
      .shadings
      .get_shading_boxed(object.shading)
      .apply(pass, resources);

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

    // // shading bind
    // let program = shading.gpu();
    // pass.use_program(program.program());

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
