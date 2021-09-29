use rendiation_webgpu::*;

pub trait RenderGraphPass {
  type Resource;
  fn execute(&self, res: &mut Self::Resource, gpu: &mut GPU);
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

  fn execute(&self, res: &mut Self::Resource, gpu: &mut GPU) {
    todo!()
  }
}

pub struct RenderGraph<R> {
  graph: Vec<Box<dyn RenderGraphPass<Resource = R>>>,
}

pub struct ResourcePool {
  pub textures: HashMap<String, Texture>,
  pub buffers: HashMap<String, Buffer>,
}

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

pub struct PassNode {
  render_by: Vec<Box<dyn Renderable>>,
}

impl PassNode {
  pub fn render_by() {
    //
  }
}

pub struct ResourceNode {
  //
}

pub struct TargetNode {
  //
}

#[rustfmt::skip]
fn demo2() {
  let mut resource = ResourcePool::new();
  let mut graph = RenderGraph::new();

  let scene_pass = graph
    .pass()
    .define_pass_ops(|b| {
      b.first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
        .depth(|d| d.load_with_clear(1.0).ok())
    })
    .render_by(&scene_main_content);

  let scene_result = graph
    .attachment()
    .with_depth()
    .size()
    .before()
    .by_pass(scene_pass);

  let high_light_pass = graph
    .pass()
    .define_pass_ops()
    .render_by(&high_light_object);

  let high_light_object_mask = graph.attachment()
    .with_depth()
    .by_pass(high_light_pass);

  let screen_compose = graph.pass()
    .define_pass_ops()
    .render_by(copy(scene_result))
    .render_by(high_light_blend(high_light_object_mask));

  graph.screen.by_pass(screen_compose);
}
