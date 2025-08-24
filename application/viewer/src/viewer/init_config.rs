use crate::*;

/// The helper struct to quickly config the default viewer behavior.
pub struct ViewerInitConfig {
  pub enable_reverse_z: bool,
  pub raster_backend_type: RasterizationRenderBackendType,
  pub prefer_bindless_for_indirect_texture_system: bool,
  pub enable_indirect_occlusion_culling: bool,
  pub transparent_config: ViewerTransparentContentRenderStyle,
}

impl Default for ViewerInitConfig {
  fn default() -> Self {
    Self {
      enable_reverse_z: true,
      raster_backend_type: RasterizationRenderBackendType::Indirect,
      prefer_bindless_for_indirect_texture_system: false,
      enable_indirect_occlusion_culling: false,
      transparent_config: ViewerTransparentContentRenderStyle::NaiveAlphaBlend,
    }
  }
}
