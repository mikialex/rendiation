use crate::{Scene, SceneGraphRenderEngine, SceneNode, RenderObject, SceneGraphBackEnd};
use rendiation::{RenderTargetAble, WGPURenderer, WGPURenderPass};

pub struct WebGPUBackend;

impl SceneGraphBackEnd for WebGPUBackend{
  // todo!();
}

pub struct SceneGraphWebGPURenderEngine {
  engine: SceneGraphRenderEngine,
}

impl SceneGraphWebGPURenderEngine {
  pub fn new() -> Self {
    Self {
      engine: SceneGraphRenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<WebGPUBackend>,
    renderer: &mut WGPURenderer,
    target: &impl RenderTargetAble,
  ) {
    self.engine.scene_raw_list.clear();
    scene.traverse(
      scene.get_root().self_id,
      |this: &mut SceneNode, parent: Option<&mut SceneNode>| {
        if let Some(parent) = parent {
          this.render_data.world_matrix =
            parent.render_data.world_matrix * this.render_data.local_matrix;
          this.net_visible = this.visible && parent.net_visible;
        }
        if !this.visible {
          return; // skip drawcall collect
        }

        this.render_objects.iter().for_each(|id| {
          self.engine.scene_raw_list.push(this.get_id(), *id);
        });
      },
    );

    scene
      .background
      .render(renderer, target.create_render_pass_builder());

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
  pub fn render_webgpu<'a, 'b: 'a>(&self, pass: &mut WGPURenderPass<'a>, scene: &'b Scene<WebGPUBackend>) {
    let shading = scene.resources.get_shading(self.shading_index);
    let geometry = scene.resources.get_geometry(self.geometry_index);

    pass.set_pipeline(shading.get_gpu_pipeline());
    pass.set_index_buffer(geometry.get_gpu_index_buffer());
    for i in 0..geometry.vertex_buffer_count() {
      let buffer = geometry.get_gpu_vertex_buffer(i);
      pass.set_vertex_buffer(i, buffer);
    }

    for i in 0..shading.get_bindgroup_count() {
      let bindgroup = scene.resources.get_bindgroup(shading.get_bindgroup(i));
      pass.set_bindgroup(i, bindgroup);
    }

    pass.draw_indexed(geometry.get_draw_range())
  }

}
