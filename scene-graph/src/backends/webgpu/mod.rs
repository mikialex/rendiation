use crate::{
  Background, RenderObject, Scene, SceneGraphBackend, RenderEngine, SolidBackground,
};
use rendiation::*;
pub mod cal;
pub use cal::*;

impl SceneGraphBackend for WebGPUBackend {
  type RenderTarget = WGPURenderPassBuilder<'static>;
  type Renderer = WGPURenderer;
  type Shading = WGPUPipeline;
  type ShadingParameterGroup = WGPUBindGroup;
  type IndexBuffer = WGPUBuffer;
  type VertexBuffer = WGPUBuffer;
  type UniformBuffer = WGPUBuffer;
  type UniformValue = ();
}

impl Background<WebGPUBackend> for SolidBackground {
  fn render(&self, renderer: &mut WGPURenderer, builder: WGPURenderPassBuilder) {
    builder
      .first_color(|c| c.load_with_clear(self.color, 1.0).ok())
      .create(&mut renderer.encoder);
  }
}

fn extend_lifetime<'b>(r: WGPURenderPassBuilder<'b>) -> WGPURenderPassBuilder<'static> {
  unsafe { std::mem::transmute::<WGPURenderPassBuilder<'b>, WGPURenderPassBuilder<'static>>(r) }
}

pub struct WebGPUBackend {
  engine: RenderEngine<WebGPUBackend>,
}

impl WebGPUBackend {
  pub fn new() -> Self {
    Self {
      engine: RenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<WebGPUBackend>,
    renderer: &mut WGPURenderer,
    target: &impl RenderTargetAble,
  ) {
    self.engine.update_render_list(scene);

    scene.background.as_ref().map(|b| {
      b.render(
        renderer,
        extend_lifetime(target.create_render_pass_builder()),
      )
    });

    let mut pass = target
      .create_render_pass_builder()
      .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
      .create(&mut renderer.encoder);

    for drawcall in &self.engine.scene_raw_list.drawcalls {
      // let node = self.nodes.get(drawcall.node).unwrap();
      let render_obj = scene.render_objects.get(drawcall.render_object).unwrap();
      render_obj.render_webgpu(&mut pass, scene);
    }
  }
}

impl RenderObject<WebGPUBackend> {
  pub fn render_webgpu<'a, 'b: 'a>(
    &self,
    pass: &mut WGPURenderPass<'a>,
    scene: &'b Scene<WebGPUBackend>,
  ) {
    let shading = scene.resources.get_shading(self.shading_index).resource();
    let geometry = scene.resources.get_geometry(self.geometry_index).resource();

    pass.set_pipeline(&shading.gpu);

    geometry.index_buffer.map(|b| {
      let index = scene.resources.get_index_buffer(b);
      pass.set_index_buffer(index.resource());
    });
    for (i, vertex_buffer) in geometry.vertex_buffers.iter().enumerate() {
      let buffer = scene.resources.get_vertex_buffer(vertex_buffer.1);
      pass.set_vertex_buffer(i, buffer.resource());
    }

    for i in 0..shading.get_parameters_count() {
      let bindgroup = scene
        .resources
        .get_shading_param_group(shading.get_parameter(i))
        .resource();
      pass.set_bindgroup(i, &bindgroup.gpu);
    }

    pass.draw_indexed(geometry.draw_range.clone())
  }
}
