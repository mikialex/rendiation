// use crate::{Scene, SceneGraphRenderEngine, RenderObject, SceneNode, SceneGraphBackEnd};


// pub struct WebGLBackend;

// impl SceneGraphBackEnd for WebGLBackend{
//   type Renderer = WebGLRenderer;
//   type Shading = WebGLProgram;
//   type ShadingParameterGroup = ();
//   type IndexBuffer = WebGLBuffer;
//   type VertexBuffer = WebGLBuffer;
// }

// pub struct WebGLRenderer{

// }

// pub struct SceneGraphWebGLEngine{
//   engine: SceneGraphRenderEngine
// }

// impl SceneGraphWebGLEngine {
//   pub fn new() -> Self {
//     Self {
//       engine: SceneGraphRenderEngine::new(),
//     }
//   }

//   // pub fn render(
//   //   &mut self,
//   //   scene: &mut Scene,
//   //   renderer: &mut WebGLRenderer,
//   //   target: &impl RenderTargetAble,
//   // ) {
//   //   self.engine.scene_raw_list.clear();
//   //   scene.traverse(
//   //     scene.get_root().self_id,
//   //     |this: &mut SceneNode, parent: Option<&mut SceneNode>| {
//   //       if let Some(parent) = parent {
//   //         this.render_data.world_matrix =
//   //           parent.render_data.world_matrix * this.render_data.local_matrix;
//   //         this.net_visible = this.visible && parent.net_visible;
//   //       }
//   //       if !this.visible {
//   //         return; // skip drawcall collect
//   //       }

//   //       this.render_objects.iter().for_each(|id| {
//   //         self.engine.scene_raw_list.push(this.get_id(), *id);
//   //       });
//   //     },
//   //   );

//   //   scene
//   //     .background
//   //     .render(renderer, target.create_render_pass_builder());

//   //   let mut pass = target
//   //     .create_render_pass_builder()
//   //     .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
//   //     .create(&mut renderer.encoder);

//   //   for drawcall in &self.engine.scene_raw_list.drawcalls {
//   //     // let node = self.nodes.get(drawcall.node).unwrap();
//   //     let render_obj = scene.render_objects.get(drawcall.render_object).unwrap();
//   //     render_obj.render_webgpu(&mut pass, scene);
//   //   }
//   // }
// }

// impl RenderObject {
//   pub fn render_webgl<'a, 'b: 'a>(&self, renderer: WebGLRenderer, scene: &'b Scene<WebGLBackend>) {
//   }

// }
