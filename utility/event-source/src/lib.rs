#![feature(type_alias_impl_trait)]

use core::{
  pin::Pin,
  task::{Context, Poll, Waker},
};
use std::sync::{Arc, Weak};

use futures::Stream;
use futures::StreamExt;
use parking_lot::*;

mod channel_like;
pub use channel_like::*;

mod channel_single;
pub use channel_single::*;

mod source;
pub use source::*;

#[macro_export]
macro_rules! noop_ctx {
  ($ctx_name: tt) => {
    let ___waker = futures::task::noop_waker_ref();
    let mut $ctx_name = std::task::Context::from_waker(___waker);
    let $ctx_name = &mut $ctx_name;
  };
}
