use crate::{
  Background, Geometry, RenderObject, Scene, SceneGraphBackEnd, SceneGraphRenderEngine,
  SolidBackground,
};
use rendiation::*;

impl SceneGraphBackEnd for SceneGraphWebGPURendererBackend {
  type Renderer = WGPURenderer;
  type Shading = WGPUPipeline;
  type ShadingParameterGroup = WGPUBindGroup;
  type IndexBuffer = WGPUBuffer;
  type VertexBuffer = WGPUBuffer;
}

impl Background<SceneGraphWebGPURendererBackend> for SolidBackground {
  fn render(&self, renderer: &mut WGPURenderer, builder: WGPURenderPassBuilder) {
    builder
      .first_color(|c| c.load_with_clear(self.color, 1.0).ok())
      .create(&mut renderer.encoder);
  }
}

pub struct SceneGraphWebGPURendererBackend {
  engine: SceneGraphRenderEngine,
}

impl SceneGraphWebGPURendererBackend {
  pub fn new() -> Self {
    Self {
      engine: SceneGraphRenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<SceneGraphWebGPURendererBackend>,
    renderer: &mut WGPURenderer,
    target: &impl RenderTargetAble,
  ) {
    self.engine.update_render_list(scene);

    scene
      .background
      .as_ref()
      .map(|b| b.render(renderer, target.create_render_pass_builder()));

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

impl RenderObject {
  pub fn render_webgpu<'a, 'b: 'a>(
    &self,
    pass: &mut WGPURenderPass<'a>,
    scene: &'b Scene<SceneGraphWebGPURendererBackend>,
  ) {
    let shading = scene.resources.get_shading(self.shading_index);
    let geometry = &scene.resources.get_geometry(self.geometry_index).data;

    pass.set_pipeline(shading.gpu());

    pass.set_index_buffer(geometry.get_gpu_index_buffer());
    for i in 0..geometry.vertex_buffer_count() {
      let buffer = geometry.get_gpu_vertex_buffer(i);
      pass.set_vertex_buffer(i, buffer);
    }

    for i in 0..shading.get_parameters_count() {
      let bindgroup = scene
        .resources
        .get_shading_param_group(shading.get_parameter(i));
      pass.set_bindgroup(i, bindgroup.gpu());
    }

    pass.draw_indexed(geometry.get_draw_range())
  }
}

use rendiation::geometry::*;
use std::ops::Range;
impl<T: PrimitiveTopology + 'static> Geometry<SceneGraphWebGPURendererBackend> for GPUGeometry<T> {
  fn update_gpu(&mut self, renderer: &mut WGPURenderer) {
    self.update_gpu(renderer)
  }

  fn get_gpu_index_buffer(&self) -> &WGPUBuffer {
    self.get_index_buffer_unwrap()
  }

  fn get_gpu_vertex_buffer(&self, _index: usize) -> &WGPUBuffer {
    self.get_vertex_buffer_unwrap()
  }

  fn get_draw_range(&self) -> Range<u32> {
    self.get_draw_range()
  }
  fn vertex_buffer_count(&self) -> usize {
    1
  }
}
