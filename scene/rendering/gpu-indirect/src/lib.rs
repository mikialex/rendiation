#![feature(stmt_expr_attributes)]

use database::*;
use fast_hash_collection::*;
use reactive::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_punctual::*;
use rendiation_scene_core::*;
pub use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod node;
pub use node::*;

mod light;
pub use light::*;

mod material;
pub use material::*;

mod scene_model;
pub use scene_model::*;

mod std_model;
pub use std_model::*;

mod rendering;
pub use rendering::*;

both!(IndirectSceneAbstractMaterialId, u32);
both!(IndirectSceneAbstractMeshId, u32);
