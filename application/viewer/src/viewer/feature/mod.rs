mod camera_control;
pub use camera_control::*;
mod gizmo_bridge;
pub use gizmo_bridge::*;
mod camera_motion;
pub use camera_motion::*;
mod pick_scene;
pub use pick_scene::*;
mod camera_helper;
pub use camera_helper::*;
mod camera_view_sync;
pub use camera_view_sync::*;
mod camera_proj_switch;
pub use camera_proj_switch::*;
mod light_helper;
pub use light_helper::*;
mod animation_player;
pub use animation_player::*;
mod gltf_io;
pub use gltf_io::*;
mod obj_io;
pub use obj_io::*;
mod screenshot;
pub use screenshot::*;
mod egui_view;
pub use egui_view::*;
mod mesh_tools;
pub use mesh_tools::*;
mod test_content;
use serde::*;
pub use test_content::*;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ViewerAppFeaturesConfig {
  pub pick_scene: PickScenePersistConfig,
}

const INIT_FILE_NAME: &str = "viewer_app_init_config.json";

impl ViewerAppFeaturesConfig {
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

  pub fn from_json_or_default(path: impl AsRef<std::path::Path>) -> Option<Self> {
    let path = path.as_ref();

    let json_file = std::fs::File::open_buffered(path)
      .inspect_err(|e| {
        log::warn!(
          "failed to read app feature config from {:?}, error: {e:?}",
          path
        )
      })
      .ok()?;

    serde_json::from_reader(json_file)
      .inspect(|_| log::info!("successfully load app feature config from {:?}", path))
      .inspect_err(|e| {
        log::warn!(
          "failed to parse app feature  config from {:?}, error: {e:?}",
          path
        )
      })
      .ok()
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PickScenePersistConfig {
  /// prefer gpu picking for nearest hit query if target platform has correct support
  pub prefer_gpu_picking: bool,
  pub enable_hit_debug_log: bool,
  pub range_query_contains: bool,
  /// compute and cache frustum edge/corner data for exact SAT intersection tests;
  /// disabling reduces per-frame cost at the expense of conservative results
  pub precise_intersection_test: bool,
}

impl Default for PickScenePersistConfig {
  fn default() -> Self {
    Self {
      prefer_gpu_picking: true,
      enable_hit_debug_log: true,
      range_query_contains: false,
      precise_intersection_test: true,
    }
  }
}
