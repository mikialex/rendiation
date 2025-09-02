use std::path::Path;

use crate::*;

/// The helper struct to quickly config the default viewer behavior.
#[derive(Serialize, Deserialize)]
pub struct ViewerInitConfig {
  pub enable_reverse_z: bool,
  pub raster_backend_type: RasterizationRenderBackendType,
  pub prefer_bindless_for_indirect_texture_system: bool,
  pub enable_indirect_occlusion_culling: bool,
  pub transparent_config: ViewerTransparentContentRenderStyle,
}

impl ViewerInitConfig {
  pub fn from_default_json_or_default() -> Self {
    let path = std::env::current_dir()
      .unwrap()
      .join("viewer_init_config.json");
    Self::from_json_or_default(path).unwrap_or_default()
  }

  pub fn export_to_current_dir(&self) {
    let path = std::env::current_dir()
      .unwrap()
      .join("viewer_init_config.json");
    let json_file = std::fs::File::create_buffered(path).unwrap();
    serde_json::to_writer_pretty(json_file, self).unwrap();
  }

  pub fn from_json_or_default(path: impl AsRef<Path>) -> Option<Self> {
    let json_file = std::fs::File::open_buffered(path).ok()?;
    serde_json::from_reader(json_file).ok()
  }
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
