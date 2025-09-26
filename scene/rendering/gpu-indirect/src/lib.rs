#![feature(stmt_expr_attributes)]
#![feature(impl_trait_in_assoc_type)]

use std::any::Any;
use std::hash::Hash;
use std::hash::Hasher;

use database::*;
use fast_hash_collection::FastHashMap;
use rendiation_device_parallel_compute::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_punctual::*;
use rendiation_scene_core::*;
pub use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;
use serde::*;

mod host_driven;

mod node;
pub use node::*;

mod mesh;
pub use mesh::*;

mod light;
pub use light::*;

mod material;
pub use material::*;

mod scene_model;
pub use scene_model::*;

mod std_model;
pub use std_model::*;

mod shape;
pub use shape::*;

mod scene;
pub use scene::*;
