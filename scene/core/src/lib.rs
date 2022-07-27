pub mod scene;
pub use scene::*;

pub mod node;
pub use node::*;

pub mod material;
pub use material::*;

pub mod background;
pub use background::*;

pub mod model;
pub use model::*;

pub mod lights;
pub use lights::*;

pub mod camera;
pub use camera::*;

pub mod identity;
pub use identity::*;

use std::{
  collections::{HashMap, HashSet},
  marker::PhantomData,
  ops::Deref,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
  },
};
