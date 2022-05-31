#![feature(capture_disjoint_fields)]
#![feature(specialization)]
#![feature(array_methods)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![feature(trait_upcasting)]
#![feature(explicit_generic_args_with_impl_trait)]
#![allow(incomplete_features)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::unit_arg)]

pub mod util;
pub use util::*;

pub use rendiation_scene_core::*;
pub use util::*;

pub use arena::*;
pub use arena_tree::*;

use bytemuck::*;
use shadergraph::*;

#[derive(Copy, Clone)]
pub struct WebGPUScene;
impl SceneContent for WebGPUScene {
  type BackGround = Box<dyn WebGPUBackground>;
  type Model = Box<dyn SceneRenderableShareable>;
  type Light = Box<dyn SceneRenderableShareable>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
}
