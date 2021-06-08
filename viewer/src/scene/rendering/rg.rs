use crate::renderer::Renderer;

pub trait RenderGraphPass {
  type Resource;
  fn execute(&self, res: &mut Self::Resource, renderer: &mut Renderer);
}

pub struct QuadPass<T> {
  material: T,
}

pub struct PostEffect {
  pipeline: wgpu::RenderPipeline,
  bindgroup: wgpu::BindGroup,
}

impl<T> RenderGraphPass for QuadPass<T> {
  type Resource = AttachmentsPool;

  fn execute(&self, res: &mut Self::Resource, renderer: &mut Renderer) {
    todo!()
  }
}

pub struct RenderGraph<R> {
  graph: Vec<Box<dyn RenderGraphPass<Resource = R>>>,
}

pub struct AttachmentsPool {}

// fn demo() {
//   let mut graph = RenderGraph::new();
//   let scene_pass = graph
//     .pass()
//     .define_pass_ops(|b| {
//       b.first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
//         .depth(|d| d.load_with_clear(1.0).ok())
//     })
//     .render_by(&scene_main_content);
// }
