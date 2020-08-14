use crate::{RenderEngine, RenderObject, Scene, ShadingParameterType};
use rendiation_webgl::WebGLRenderer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebGLBackend {
  engine: RenderEngine<WebGLRenderer>,
}

impl WebGLBackend {
  pub fn new() -> Self {
    Self {
      engine: RenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<WebGLRenderer>,
    renderer: &mut WebGLRenderer,
    target: <WebGLRenderer as rendiation_ral::RALBackend>::RenderTarget,
  ) {
    self.engine.update_render_list(scene);

    scene
      .background
      .as_ref()
      .map(|b| b.render(renderer, target));

    for drawcall in &self.engine.scene_raw_list.drawcalls {
      // let node = self.nodes.get(drawcall.node).unwrap();
      let render_obj = scene.render_objects.get(drawcall.render_object).unwrap();
      render_obj.render_webgl(renderer, scene);
    }
  }
}

impl RenderObject<WebGLRenderer> {
  pub fn render_webgl(&self, renderer: &mut WebGLRenderer, scene: &Scene<WebGLRenderer>) {
    let resources = &scene.resources;
    let shading = resources.get_shading(self.shading_index).resource();
    let geometry = &resources.get_geometry(self.geometry_index).resource();
    let program = shading.gpu();

    renderer.use_program(program.program());

    // geometry bind
    renderer.attribute_states.prepare_new_bindings();
    geometry.index_buffer.map(|b| {
      let index = resources.get_index_buffer(b);
      renderer.set_index_buffer(index.resource().as_ref());
    });
    geometry.vertex_buffers.iter().for_each(|v| {
      let buffer = resources.get_vertex_buffer(v.1).resource();
      let att_location = program.query_attribute_location(v.0);
      renderer.set_vertex_buffer(att_location, buffer);
    });
    renderer
      .attribute_states
      .disable_old_unused_bindings(&renderer.gl);

    // shading bind
    renderer.texture_slot_states.reset_slots();
    for i in 0..shading.get_parameters_count() {
      let parameter_group = resources
        .get_shading_param_group(shading.get_parameter(i))
        .resource();
      parameter_group.items().iter().for_each(|p| {
        use ShadingParameterType::*;
        match p.1 {
          UniformBuffer(index) => {
            let _uniform = resources.get_uniform(index).resource();
            todo!()
          }
          UniformValue(index) => {
            let uniform_value = resources.get_uniform_value(index).resource();
            program.upload_uniform_value(uniform_value, p.0, &renderer.gl);
          }
          SampledTexture(_) => {
            // let texture = resources.get_sampled_texture(index).respirce();
          }
          _ => panic!("unsupported webgl resource type"),
        }
      })
    }

    todo!()
    // let range = &geometry.draw_range;
    // renderer.gl.draw_elements_with_i32(
    //   WebGl2RenderingContext::TRIANGLES,
    //   range.start as i32,
    //   WebGl2RenderingContext::UNSIGNED_INT,
    //   range.end as i32,
    // );
  }
}
