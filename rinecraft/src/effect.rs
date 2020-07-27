use rendiation_rendergraph::*;
use rendiation_webgpu::{ScreenRenderTargetInstance, WGPURenderer};
use std::collections::HashMap;

pub struct EffectManager {
  cache: HashMap<EffectConfig, RenderGraph<WebGPURenderGraphBackend>>,
  executor: RenderGraphExecutor<WebGPURenderGraphBackend>,
}

impl EffectManager {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
      executor: RenderGraphExecutor::new(),
    }
  }

  fn render(
    &mut self,
    renderer: &mut WGPURenderer,
    target: &ScreenRenderTargetInstance,
    config: &EffectConfig,
  ) {
    let graph = self
      .cache
      .entry(*config)
      .or_insert_with(|| EffectManager::build(config));
    let target = unsafe { std::mem::transmute(&target) };
    self.executor.render(graph, target, renderer);
  }

  fn build(config: &EffectConfig) -> RenderGraph<WebGPURenderGraphBackend> {
    let graph = RenderGraph::new();
    let normal_pass = graph.pass("normal");
    let normal_target = graph.target("normal").from_pass(&normal_pass);
    let copy_screen = graph
      .pass("copy_screen")
      .depend(&normal_target)
      .render_by(|_, _| {
        let _a = 1;
      });
    graph.finally().from_pass(&copy_screen);
    graph
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectConfig {
  enable_grain: bool,
}
