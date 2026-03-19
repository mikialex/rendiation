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
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_shadow_map::*;
use rendiation_lighting_transport::*;
use rendiation_mesh_core::*;
use rendiation_mesh_lod_graph_rendering::*;
pub use rendiation_mesh_lod_graph_rendering::{
  DefaultMeshLODBuilder, LODGraphData, LODGraphMeshEntity, MeshLODGraph, MeshLodGraphBuilder,
  StandardModelRefLodGraphMeshEntity,
};
use rendiation_scene_batch_extractor::*;
pub use rendiation_scene_core::*;
pub use rendiation_scene_geometry_query::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_scene_scheduler::*;
use rendiation_shader_api::*;
pub use rendiation_texture_core::Size;
use rendiation_texture_core::*;
use rendiation_texture_gpu_base::{create_gpu_texture2d, SamplerConvertExt};
use rendiation_texture_gpu_process::{ToneMap, ToneMapType};
pub use rendiation_webgpu::raw_gpu;
use rendiation_webgpu::*;
pub use rendiation_webgpu::{CreateSurfaceError, GPUInstance, GPUSurface, SurfaceProvider, GPU};
use rendiation_webgpu_hook_utils::*;
use rendiation_webgpu_virtual_typed_combine_buffer::*;
use rendiation_wide_line::*;
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
pub use viewer::*;
pub use viewport::*;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

pub struct Viewer3dContent {
  pub viewports: Vec<ViewerViewPort>,
  pub root: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub selected_model: Option<EntityHandle<SceneModelEntity>>,
  pub selected_dir_light: Option<EntityHandle<DirectionalLightEntity>>,
  pub selected_spot_light: Option<EntityHandle<SpotLightEntity>>,
  pub selected_point_light: Option<EntityHandle<PointLightEntity>>,
  pub widget_scene: EntityHandle<SceneEntity>,
}

pub fn register_viewer_content_data_model() {
  register_area_lighting_data_model();
  register_scene_mesh_lod_graph_data_model(true);
  register_sky_env_data_model();
}
