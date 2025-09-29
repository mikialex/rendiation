use std::{num::NonZeroU32, path::Path};

use rendiation_lighting_shadow_map::MultiLayerTexturePackerConfig;
use rendiation_scene_rendering_gpu_indirect::BindlessMeshInit;

use crate::*;

/// The helper struct to quickly config the default viewer behavior.
#[derive(Serialize, Deserialize)]
#[serde(default)] // any missing field will be set to the struct's default
pub struct ViewerInitConfig {
  pub raster_backend_type: RasterizationRenderBackendType,
  pub prefer_bindless_for_indirect_texture_system: bool,
  pub enable_indirect_occlusion_culling: bool,
  pub using_host_driven_indirect_draw: bool,
  pub transparent_config: ViewerTransparentContentRenderStyle,
  pub present_mode: PresentMode,
  pub init_only: ViewerStaticInitConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)] // any missing field will be set to the struct's default
/// configs that can not be changed dynamically in runtime
pub struct ViewerStaticInitConfig {
  pub texture_pool_source_init_config: TexturePoolSourceInit,
  /// None means use available parallelism, 1 means no parallelism
  pub thread_pool_thread_count: Option<usize>,
  pub bindless_mesh_init: BindlessMeshInit,
  pub enable_indirect_storage_combine: bool,
  pub enable_reverse_z: bool,
  /// if not provided, the backend select will be automatically based on platform available
  pub wgpu_backend_select_override: Option<Backends>,
  pub using_texture_as_storage_buffer_for_indirect_rendering: bool,
}

impl Default for ViewerStaticInitConfig {
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
      texture_pool_source_init_config: init,
      thread_pool_thread_count: None,
      bindless_mesh_init: Default::default(),
      wgpu_backend_select_override: None,
      enable_indirect_storage_combine: true,
      using_texture_as_storage_buffer_for_indirect_rendering: false,
    }
  }
}

const INIT_FILE_NAME: &str = "viewer_init_config.json";

impl ViewerInitConfig {
  pub fn from_default_json_or_default() -> Self {
    #[cfg(not(target_family = "wasm"))]
    return {
      let path = std::env::current_dir().unwrap().join(INIT_FILE_NAME);
      Self::from_json_or_default(path).unwrap_or_default()
    };

    #[cfg(target_family = "wasm")]
    Self::default()
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
    Self {
      present_mode: PresentMode::AutoVsync,
      raster_backend_type: RasterizationRenderBackendType::Gles,
      prefer_bindless_for_indirect_texture_system: false,
      enable_indirect_occlusion_culling: false,
      using_host_driven_indirect_draw: false,
      transparent_config: ViewerTransparentContentRenderStyle::NaiveAlphaBlend,
      init_only: ViewerStaticInitConfig::default(),
    }
  }
}
