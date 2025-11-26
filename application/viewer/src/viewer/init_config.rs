use std::{num::NonZeroU32, path::Path};

use rendiation_lighting_shadow_map::MultiLayerTexturePackerConfig;
use rendiation_scene_rendering_gpu_indirect::BindlessMeshInit;

use crate::*;

/// The helper struct to quickly config the default viewer behavior.
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)] // any missing field will be set to the struct's default
pub struct ViewerInitConfig {
  pub raster_backend_type: RasterizationRenderBackendType,
  pub prefer_bindless_for_indirect_texture_system: bool,
  pub enable_indirect_occlusion_culling: bool,
  pub enable_debug_cull_result: bool,
  pub enable_frustum_culling: bool,
  pub using_host_driven_indirect_draw: bool,
  pub transparent_config: ViewerTransparentContentRenderStyle,
  pub enable_shadow: bool,
  pub present_mode: PresentMode,
  pub enable_on_demand_rendering: bool,
  pub init_only: ViewerStaticInitConfig,
  pub features: ViewerFeaturesInitConfig,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ViewerFeaturesInitConfig {
  pub pick_scene: PickScenePersistConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)] // any missing field will be set to the struct's default
/// configs that can not be changed dynamically in runtime
pub struct ViewerStaticInitConfig {
  pub texture_pool_source_init_config: TexturePoolSourceInit,
  /// None means use available parallelism, 1 means no parallelism
  pub thread_pool_thread_count: Option<usize>,
  pub occlusion_culling_max_scene_model_count: u32,
  pub bindless_mesh_init: BindlessMeshInit,
  pub enable_indirect_storage_combine: bool,
  pub enable_reverse_z: bool,
  /// if not provided, the backend select will be automatically based on platform available
  pub wgpu_backend_select_override: Option<Backends>,
  pub using_texture_as_storage_buffer_for_indirect_rendering: bool,
  pub default_shader_protections: ShaderRuntimeProtection,
  /// if None, then using wgpu default behavior (on when debug build)
  ///
  /// this is useful if we want to disable validation in debug to improve debug build performance
  /// or do extra debug check in release build
  pub enable_backend_validation: Option<bool>,

  /// the dxc dll path for dx12 backend, the dll must support shader model 6.7 at least
  /// if None, then using fxc compiler, which is buggy.
  pub dx_compiler_dll_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ShaderRuntimeProtection {
  pub bounds_checks: bool,
  pub force_loop_bounding: bool,
}

impl Default for ViewerStaticInitConfig {
  fn default() -> Self {
    // this should passed in by user
    let size = Size::from_u32_pair_min_one((4096, 4096));
    let init = TexturePoolSourceInit {
      init_texture_count_capacity: 128,
      init_sampler_count_capacity: 128,
      atlas_config: MultiLayerTexturePackerConfig {
        max_size: SizeWithDepth {
          depth: NonZeroU32::new(16).unwrap(),
          size,
        },
        init_size: SizeWithDepth {
          depth: NonZeroU32::new(2).unwrap(),
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
      occlusion_culling_max_scene_model_count: u16::MAX as u32,
      using_texture_as_storage_buffer_for_indirect_rendering: false,
      default_shader_protections: ShaderRuntimeProtection {
        bounds_checks: true,
        force_loop_bounding: true,
      },
      enable_backend_validation: None,
      dx_compiler_dll_path: None,
    }
  }
}

pub const INIT_FILE_NAME: &str = "viewer_init_config.json";

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

  #[cfg(not(target_family = "wasm"))]
  pub fn export_to_current_dir(&self) {
    let path = std::env::current_dir().unwrap().join(INIT_FILE_NAME);
    let json_file = std::fs::File::create_buffered(path).unwrap();
    serde_json::to_writer_pretty(json_file, self).unwrap();
  }

  #[cfg(target_family = "wasm")]
  pub fn export_to_current_dir(&self) {
    let config_str = serde_json::to_string_pretty(self).unwrap();
    log::info!("{}", config_str);
  }

  pub fn from_json_or_default(path: impl AsRef<Path>) -> Option<Self> {
    let path = path.as_ref();

    let json_file = std::fs::File::open_buffered(path)
      .inspect_err(|e| log::warn!("failed to read config from {:?}, error: {e:?}", path))
      .ok()?;

    serde_json::from_reader(json_file)
      .inspect(|_| log::info!("successfully load config from {:?}", path))
      .inspect_err(|e| log::warn!("failed to parse config from {:?}, error: {e:?}", path))
      .ok()
  }
}

impl Default for ViewerInitConfig {
  fn default() -> Self {
    Self {
      present_mode: PresentMode::AutoVsync,
      raster_backend_type: RasterizationRenderBackendType::Gles,
      prefer_bindless_for_indirect_texture_system: false,
      enable_indirect_occlusion_culling: false,
      enable_debug_cull_result: false,
      enable_frustum_culling: true,
      enable_shadow: true,
      using_host_driven_indirect_draw: false,
      enable_on_demand_rendering: true,
      transparent_config: ViewerTransparentContentRenderStyle::NaiveAlphaBlend,
      init_only: ViewerStaticInitConfig::default(),
      features: Default::default(),
    }
  }
}
