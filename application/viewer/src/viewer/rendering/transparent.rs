use rendiation_oit::OitLoop32Renderer;

use crate::*;

#[derive(Serialize, Deserialize)]
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
