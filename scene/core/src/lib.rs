#![feature(let_chains)]

pub mod scene;
pub use scene::*;

pub mod node;
pub use node::*;

pub mod ext;
pub use ext::*;

pub mod mesh;
pub use mesh::*;

pub mod material;
pub use material::*;

pub mod texture;
pub use texture::*;

pub mod background;
pub use background::*;

pub mod model;
pub use model::*;

pub mod light;
pub use light::*;

pub mod camera;
pub use camera::*;

pub mod animation;
pub use animation::*;

mod utils;
pub use utils::*;

mod system;
pub use system::*;

use incremental::*;
use rendiation_algebra::*;
use std::any::Any;
use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};
