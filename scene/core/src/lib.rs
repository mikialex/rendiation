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

pub mod identity;
pub use identity::*;
// mod identity_next;

use incremental::*;
use rendiation_algebra::*;
use std::any::Any;
use std::{
  collections::HashMap,
  marker::PhantomData,
  ops::Deref,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
  },
};
