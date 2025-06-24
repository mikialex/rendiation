use std::sync::Arc;

use parking_lot::RwLock;
use rendiation_oit::OitLoop32Renderer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerTransparentContentRenderStyle {
  NaiveAlphaBlend,
  Loop32OIT,
  WeightedOIT,
}

#[derive(Clone)]
pub enum ViewerTransparentRenderer {
  NaiveAlphaBlend,
  Loop32OIT(Arc<RwLock<OitLoop32Renderer>>),
  WeightedOIT,
}
