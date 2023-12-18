use std::task::Waker;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use rayon::prelude::*;

use crate::*;

pub fn multi_join_updates(tasks: Vec<Task>, cx: &mut Context) {
  let (retry_sender, rev) = futures::channel::mpsc::unbounded();
  for t in tasks {
    retry_sender.unbounded_send(t).ok();
  }
  WorkingList {
    rev,
    retry_sender,
    waker: cx.waker().clone(),
  }
  .par_bridge()
  .for_each(|v| v.run());
}

type Task<'a> = &'a mut (dyn FnMut(&mut Context) -> CPoll<()> + Send);

struct TaskWrapper<'a> {
  task: Task<'a>,
  retry_sender: UnboundedSender<Task<'a>>,
  waker: Waker,
}

impl<'a> TaskWrapper<'a> {
  fn run(self) {
    let mut cx = Context::from_waker(&self.waker);
    if let CPoll::Blocked = (self.task)(&mut cx) {
      self.retry_sender.unbounded_send(self.task).ok();
    }
  }
}

struct WorkingList<'a> {
  rev: UnboundedReceiver<Task<'a>>,
  retry_sender: UnboundedSender<Task<'a>>,
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
