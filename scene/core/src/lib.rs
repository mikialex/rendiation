#![feature(type_alias_impl_trait)]
#![feature(stmt_expr_attributes)]
#![allow(incomplete_features)]
#![feature(iterator_try_collect)]
#![feature(impl_trait_in_assoc_type)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(let_chains)]

 mod scene;
pub use scene::*;

 mod node;
pub use node::*;

 mod ext;
pub use ext::*;

 mod mesh;
pub use mesh::*;
 mod mesh_picking;
pub use mesh_picking::*;
mod mesh_merge;
pub use mesh_merge::*;

mod material;
pub use material::*;

mod texture;
pub use texture::*;

mod background;
pub use background::*;

mod model;
pub use model::*;

mod light;
pub use light::*;

mod camera;
pub use camera::*;

mod animation;
pub use animation::*;

mod utils;
pub use utils::*;

mod systems;
use std::any::Any;
use std::hash::Hash;
use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

pub use dyn_downcast::*;
use futures::Stream;
use incremental::*;
use rendiation_algebra::*;
pub use systems::*;
