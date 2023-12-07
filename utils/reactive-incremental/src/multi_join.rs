use std::task::Waker;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use rayon::prelude::*;

use crate::*;

pub trait ReactiveUpdateLogic: Send {
  fn poll_update(&mut self, cx: &mut Context) -> CPoll<()>;
}

impl<'a, K, V> ReactiveUpdateLogic for &'a mut dyn DynamicReactiveCollection<K, V> {
  fn poll_update(&mut self, cx: &mut Context) -> CPoll<()> {
    self.poll_changes_dyn(cx).map(|_| {})
  }
}

pub fn multi_join_updates(tasks: Vec<&mut dyn ReactiveUpdateLogic>, cx: &mut Context) {
  let (retry_sender, rev) = futures::channel::mpsc::unbounded();
  for t in tasks {
    retry_sender.unbounded_send(t).ok();
  }
  WorkingList {
    rev,
    retry_sender,
    waker: cx.waker().clone(),
  }
  .map(|v| v.run())
  .par_bridge();
}

struct TaskWrapper<'a> {
  task: &'a mut dyn ReactiveUpdateLogic,
  retry_sender: UnboundedSender<&'a mut dyn ReactiveUpdateLogic>,
  waker: Waker,
}

impl<'a> TaskWrapper<'a> {
  fn run(self) {
    let mut cx = Context::from_waker(&self.waker);
    if let CPoll::Blocked = self.task.poll_update(&mut cx) {
      self.retry_sender.unbounded_send(self.task).ok();
    }
  }
}

struct WorkingList<'a> {
  rev: UnboundedReceiver<&'a mut dyn ReactiveUpdateLogic>,
  retry_sender: UnboundedSender<&'a mut dyn ReactiveUpdateLogic>,
  waker: Waker,
}

impl<'a> Iterator for WorkingList<'a> {
  type Item = TaskWrapper<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    let waker = futures::task::noop_waker_ref();
    let mut cx = Context::from_waker(waker);
    if let Poll::Ready(Some(r)) = self.rev.poll_next_unpin(&mut cx) {
      Some(TaskWrapper {
        task: r,
        retry_sender: self.retry_sender.clone(),
        waker: self.waker.clone(),
      })
    } else {
      None
    }
  }
}
