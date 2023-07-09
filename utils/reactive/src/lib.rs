#![feature(type_alias_impl_trait)]

use core::{
  pin::Pin,
  task::{Context, Poll, Waker},
};
use std::sync::{Arc, Mutex, RwLock, Weak};

use fast_hash_collection::*;
use futures::stream::FusedStream;
use futures::Stream;
use futures::StreamExt;
use pin_project::pin_project;

mod signal_stream;
pub use signal_stream::*;

mod vec;
pub use vec::*;

mod poll_utils;
pub use poll_utils::*;

mod channel_like;
pub use channel_like::*;

mod channel_single;
pub use channel_single::*;

mod channel_batch;
pub use channel_batch::*;

mod source;
pub use source::*;

mod broadcast;
pub use broadcast::*;

mod map;
pub use map::*;
