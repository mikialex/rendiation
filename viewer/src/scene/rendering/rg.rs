pub trait RenderGraphPass {
  fn execute(&self);
}

pub struct RenderGraph {
  graph: Vec<Box<dyn RenderGraphPass>>,
}
