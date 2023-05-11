#![feature(type_alias_impl_trait)]

mod signal_stream;
pub use signal_stream::*;

mod vec;
pub use vec::*;

mod channel;
pub use channel::*;

mod channel_like;
pub use channel_like::*;

mod source;
pub use source::*;

mod buff_shared;
pub use buff_shared::*;

mod broadcast;
pub use broadcast::*;

mod map;
use core::{
  pin::Pin,
  task::{Context, Poll, Waker},
};
use std::sync::{Arc, Mutex, RwLock, Weak};

use futures::Stream;
pub use map::*;
use pin_project::pin_project;
