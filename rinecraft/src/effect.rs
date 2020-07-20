use rendiation_rendergraph::*;
use std::collections::HashMap;

pub struct EffectManager {
  cache: HashMap<EffectConfig, RenderGraph<WebGPURenderGraphBackend>>,
}

impl EffectManager{
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
    }
  }

  fn get_graph(&mut self, config: &EffectConfig) {
    
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectConfig {
  enable_grain: bool,
}
