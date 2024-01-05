#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]
#![feature(impl_trait_in_assoc_type)]
#![feature(let_chains)]

use std::hash::Hash;
use std::sync::{Arc, RwLock};

use derivative::Derivative;
pub use dyn_downcast::*;
use fast_hash_collection::*;
use futures::Stream;
use incremental::*;
pub use reactive_incremental::*;
use rendiation_algebra::*;
pub use rendiation_mesh_core::*;

mod scene;
pub use scene::*;

mod node;
pub use node::*;

mod ext;
pub use ext::*;

mod mesh;
pub use mesh::*;

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

mod systems;
pub use systems::*;

mod systems_next;
pub use systems_next::*;

pub type ForeignObject = Box<dyn AnyClone + Send + Sync>;

fn byte_hash<T: bytemuck::Pod, H>(value: &T, state: &mut H)
where
  H: std::hash::Hasher,
{
  bytemuck::bytes_of(value).hash(state)
}
