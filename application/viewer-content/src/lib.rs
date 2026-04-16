#![feature(impl_trait_in_assoc_type)]
#![feature(file_buffered)]

use std::any::Any;
use std::future::Future;
use std::hash::Hash;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::task::Waker;

pub use database::*;
use event_source::*;
use fast_hash_collection::*;
use futures::FutureExt;
use futures::StreamExt;
use parking_lot::*;
pub use rendiation_algebra::*;
use rendiation_area_lighting::*;
pub use rendiation_area_lighting::{
  AreaLightEntity, AreaLightIntensity, AreaLightIsDoubleSide, AreaLightIsRound, AreaLightRefNode,
  AreaLightRefScene, AreaLightSize,
};
use rendiation_controller::InputBound;
use rendiation_geometry::*;
pub use rendiation_gui_3d::ViewportPointerCtx;
use rendiation_gui_3d::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_shadow_map::*;
use rendiation_lighting_transport::*;
pub use rendiation_mesh_core::*;
use rendiation_mesh_lod_graph_rendering::*;
pub use rendiation_mesh_lod_graph_rendering::{
  DefaultMeshLODBuilder, LODGraphData, LODGraphMeshEntity, MeshLODGraph, MeshLodGraphBuilder,
  StandardModelRefLodGraphMeshEntity,
};
pub use rendiation_occ_style_draw_control::{
  OccStyleZLayer, SceneModelOccStyleLayer, SceneModelOccStylePriority,
};
use rendiation_scene_batch_extractor::*;
pub use rendiation_scene_core::*;
pub use rendiation_scene_geometry_query::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_scene_scheduler::*;
use rendiation_shader_api::*;
pub use rendiation_text_3d::*;
use rendiation_texture_core::*;
pub use rendiation_texture_core::{GPUBufferImage, Size};
use rendiation_texture_gpu_base::{create_gpu_texture2d, SamplerConvertExt};
use rendiation_texture_gpu_process::{ToneMap, ToneMapType};
use rendiation_view_dependent_transform::*;
pub use rendiation_view_dependent_transform::{
  OccStyleCorner, OccStyleMode, OccStyleTransform, OccStyleViewDepConfig, SceneCameraLookAt,
  SceneModelViewDependentTransformOcc,
};
pub use rendiation_webgpu::raw_gpu;
use rendiation_webgpu::*;
pub use rendiation_webgpu::{CreateSurfaceError, GPUInstance, GPUSurface, SurfaceProvider, GPU};
use rendiation_webgpu_hook_utils::*;
use rendiation_webgpu_virtual_typed_combine_buffer::*;
pub use rendiation_wide_line::*;
use rendiation_wide_styled_points::*;
pub use rendiation_wide_styled_points::{
  SceneModelWideStyledPointsRenderPayload, WideStyledPointVertex, WideStyledPointsEntity,
  WidesStyledPointsMeshBuffer,
};
use serde::{Deserialize, Serialize};
use tracing::*;

mod background;
mod bounding;
mod data_source;
mod egui_helper;
mod gpu_picker;
mod gpu_with_surface;
mod init_config;
mod pick;
mod rendering;
mod rendering_root;
mod terminal;
mod util;
mod view_dependent_transform;
mod viewer;
mod viewport;

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

pub use background::*;
pub use bounding::*;
pub use data_source::*;
pub use egui_helper::*;
pub use gpu_picker::*;
pub use gpu_with_surface::*;
pub use init_config::*;
pub use pick::*;
pub use rendering::*;
pub use rendering_root::*;
pub use terminal::*;
pub use util::*;
use view_dependent_transform::*;
pub use viewer::*;
pub use viewport::*;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

pub struct ViewerSurfaceContent {
  pub viewports: Vec<ViewerViewPort>,
  /// the viewport is physical size. we store the dpi per surface to help the convert to logic pixel
  pub device_pixel_ratio: f32,

  // the currently implementation only allows one scene for one surface, not one scene for one viewport
  pub root: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub selected_model: Option<EntityHandle<SceneModelEntity>>,
  pub selected_dir_light: Option<EntityHandle<DirectionalLightEntity>>,
  pub selected_spot_light: Option<EntityHandle<SpotLightEntity>>,
  pub selected_point_light: Option<EntityHandle<PointLightEntity>>,
  pub widget_scene: EntityHandle<SceneEntity>,
  pub background: ViewerBackgroundState,
}

pub fn register_viewer_content_data_model() {
  register_scene_core_data_model();
  register_light_shadow_config();
  register_gui3d_extension_data_model(true);
  register_clipping_data_model();
  register_area_lighting_data_model();
  register_scene_mesh_lod_graph_data_model(true);
  register_sky_env_data_model();
  register_wide_styled_points_data_model(true);
  register_text3d_data_model(true);
  register_occ_style_view_dependent_data_model();
  rendiation_occ_style_draw_control::register_occ_style_draw_control_data_model();
}
