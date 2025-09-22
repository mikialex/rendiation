use std::{num::NonZeroU32, path::Path};

use rendiation_lighting_shadow_map::MultiLayerTexturePackerConfig;
use rendiation_scene_rendering_gpu_indirect::BindlessMeshInit;

use crate::*;

/// The helper struct to quickly config the default viewer behavior.
#[derive(Serialize, Deserialize)]
#[serde(default)] // any missing field will be set to the struct's default
pub struct ViewerInitConfig {
  pub enable_reverse_z: bool,
  pub raster_backend_type: RasterizationRenderBackendType,
  pub prefer_bindless_for_indirect_texture_system: bool,
  pub enable_indirect_storage_combine: bool,
  pub enable_indirect_occlusion_culling: bool,
  pub transparent_config: ViewerTransparentContentRenderStyle,
  pub texture_pool_source_init_config: TexturePoolSourceInit,
  /// None means use available parallelism, 1 means no parallelism
  pub thread_pool_thread_count: Option<usize>,
  pub bindless_mesh_init: BindlessMeshInit,
}

const INIT_FILE_NAME: &str = "viewer_init_config.json";

impl ViewerInitConfig {
  pub fn from_default_json_or_default() -> Self {
    let path = std::env::current_dir().unwrap().join(INIT_FILE_NAME);
    Self::from_json_or_default(path).unwrap_or_default()
  }

  pub fn export_to_current_dir(&self) {
    let path = std::env::current_dir().unwrap().join(INIT_FILE_NAME);
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
    // this should passed in by user
    let size = Size::from_u32_pair_min_one((4096, 4096));
    let init = TexturePoolSourceInit {
      init_texture_count_capacity: 128,
      init_sampler_count_capacity: 128,
      format: TextureFormat::Rgba8Unorm,
      atlas_config: MultiLayerTexturePackerConfig {
        max_size: SizeWithDepth {
          depth: NonZeroU32::new(16).unwrap(),
          size,
        },
        init_size: SizeWithDepth {
          depth: NonZeroU32::new(1).unwrap(),
          size,
        },
      },
    };

    Self {
      enable_reverse_z: true,
      raster_backend_type: RasterizationRenderBackendType::Indirect,
      prefer_bindless_for_indirect_texture_system: false,
      enable_indirect_occlusion_culling: false,
      enable_indirect_storage_combine: true,
      transparent_config: ViewerTransparentContentRenderStyle::NaiveAlphaBlend,
      texture_pool_source_init_config: init,
      thread_pool_thread_count: None,
      bindless_mesh_init: Default::default(),
    }
  }
}
