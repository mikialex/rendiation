use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::channel::mpsc::*;
use futures::StreamExt;
use reactive::noop_ctx;

use crate::*;
type StatisticTaskPreOutput = (String, PipelineQueryResult, u64);
type StatisticTaskOutput = (String, Option<DeviceDrawStatistics>, u64);
type StatisticTask = Pin<Box<dyn Future<Output = StatisticTaskOutput> + Send>>;
pub type StatisticTaskPreSender = UnboundedSender<StatisticTaskPreOutput>;
pub type StatisticTaskPreReceiver = UnboundedReceiver<StatisticTaskPreOutput>;
pub type StatisticTaskSender = UnboundedSender<StatisticTask>;
type StatisticTaskReceiver = Pin<Box<dyn futures::Stream<Item = StatisticTaskOutput>>>;

pub struct FramePassStatistics {
  pub statics_task_sender: StatisticTaskSender,
  pipeline_statistics_pending: StatisticTaskReceiver,
  pub pipeline_statistics: FastHashMap<String, StatisticComputer>,
  pub max_history: usize,
}

pub struct StatisticComputer {
  /// currently we only store the history but not do any analysis
  pub history: Vec<Option<(DeviceDrawStatistics, u64)>>,
  pub latest_resolved: Option<(DeviceDrawStatistics, u64)>,
}

impl StatisticComputer {
  fn new(max_history: usize) -> Self {
    StatisticComputer {
      history: vec![None; max_history],
      latest_resolved: None,
    }
  }
  fn insert(&mut self, value: DeviceDrawStatistics, idx: u64) {
    let write_idx = idx as usize % self.history.len();
    self.history[write_idx] = Some((value, idx));
    if let Some(l) = self.latest_resolved {
      if l.1 < idx {
        self.latest_resolved = Some((value, idx));
      }
    } else {
      self.latest_resolved = Some((value, idx));
    }
  }
}

impl FramePassStatistics {
  pub fn new(max_history: usize) -> Self {
    let (sender, receiver) = unbounded();
    let receiver = receiver.buffer_unordered(64).boxed();

    Self {
      statics_task_sender: sender,
      pipeline_statistics_pending: receiver,
      pipeline_statistics: FastHashMap::default(),
      max_history,
    }
  }

  pub fn poll(&mut self, cx: &mut Context) {
    while let Poll::Ready(Some((name, Some(result), idx))) =
      self.pipeline_statistics_pending.poll_next_unpin(cx)
    {
      self
        .pipeline_statistics
        .raw_entry_mut()
        .from_key(&name)
        .or_insert_with(|| (name.clone(), StatisticComputer::new(self.max_history)))
        .1
        .insert(result, idx);
    }
  }

  pub fn clear_history(&mut self, max_history: usize) {
    self.pipeline_statistics.clear();
    self.max_history = max_history;
  }

  pub fn create_resolver(&mut self) -> FrameStaticInfoResolver {
    let (sub_pass_info_sender, sub_pass_info_receiver) = unbounded();
    FrameStaticInfoResolver {
      sub_pass_info_sender,
      sub_pass_info_receiver,
      static_output_sender: self.statics_task_sender.clone(),
    }
  }
}

pub struct FrameStaticInfoResolver {
  pub(crate) sub_pass_info_sender: StatisticTaskPreSender,
  sub_pass_info_receiver: StatisticTaskPreReceiver,
  static_output_sender: StatisticTaskSender,
}

impl FrameStaticInfoResolver {
  pub fn resolve(&mut self, gpu: &GPU, encoder: &mut GPUCommandEncoder) {
    noop_ctx!(cx);
    while let Poll::Ready(Some((name, result, idx))) =
      self.sub_pass_info_receiver.poll_next_unpin(cx)
    {
      let f = result.read_back(&gpu.device, encoder);
      let f = f.map(move |r| (name, r, idx));
      let f = Box::pin(f);
      self.static_output_sender.unbounded_send(f).ok();
    }
  }
}
