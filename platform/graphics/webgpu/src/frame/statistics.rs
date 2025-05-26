use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::channel::mpsc::*;
use futures::StreamExt;
use reactive::noop_ctx;

pub struct FramePassMeasure<T> {
  pub info: T,
  pub pass_name: String,
  pub frame_index: u64,
}

use crate::*;
type StatisticTaskOutput = Option<FramePassMeasure<DeviceDrawStatistics>>;
type TimeTaskOutput = Option<FramePassMeasure<TimestampPair>>;

type StatisticTask = Pin<Box<dyn Future<Output = StatisticTaskOutput> + Send>>;
type TimeTask = Pin<Box<dyn Future<Output = TimeTaskOutput> + Send>>;

type StatisticTaskReceiver = Pin<Box<dyn futures::Stream<Item = StatisticTaskOutput>>>;
type TimeTaskReceiver = Pin<Box<dyn futures::Stream<Item = TimeTaskOutput>>>;

pub struct FramePassStatistics {
  pub statistic_task_sender: UnboundedSender<StatisticTask>,
  pub time_output_sender: UnboundedSender<TimeTask>,
  pipeline_statistics_pending: StatisticTaskReceiver,
  time_statistics_pending: TimeTaskReceiver,
  time_unit_in_nanoseconds: f32,
  pub collected: FastHashMap<String, PassRenderStatistics>,
  pub max_history: usize,
  pub time_query_supported: bool,
  pub pipeline_query_supported: bool,
}

pub struct PassRenderStatistics {
  pub pipeline: StatisticStore<DeviceDrawStatistics>,
  pub time: StatisticStore<f64>,
}

impl PassRenderStatistics {
  pub fn new(max_history: usize) -> Self {
    Self {
      pipeline: StatisticStore::new(max_history),
      time: StatisticStore::new(max_history),
    }
  }
}

impl FramePassStatistics {
  pub fn new(max_history: usize, gpu: &GPU) -> Self {
    let (sender, receiver) = unbounded();
    let receiver = receiver.buffer_unordered(64).boxed();

    let (t_sender, t_receiver) = unbounded();
    let t_receiver = t_receiver.buffer_unordered(64).boxed();

    Self {
      time_unit_in_nanoseconds: gpu.queue.get_timestamp_period(),
      statistic_task_sender: sender,
      pipeline_statistics_pending: receiver,
      time_statistics_pending: t_receiver,
      collected: FastHashMap::default(),
      max_history,
      time_output_sender: t_sender,
      time_query_supported: gpu
        .info
        .supported_features
        .contains(Features::TIMESTAMP_QUERY),
      pipeline_query_supported: gpu
        .info
        .supported_features
        .contains(Features::PIPELINE_STATISTICS_QUERY),
    }
  }

  pub fn poll(&mut self, cx: &mut Context) {
    while let Poll::Ready(Some(Some(FramePassMeasure {
      info,
      pass_name,
      frame_index,
    }))) = self.pipeline_statistics_pending.poll_next_unpin(cx)
    {
      self
        .collected
        .raw_entry_mut()
        .from_key(&pass_name)
        .or_insert_with(|| {
          (
            pass_name.clone(),
            PassRenderStatistics::new(self.max_history),
          )
        })
        .1
        .pipeline
        .insert(info, frame_index);
    }

    while let Poll::Ready(Some(Some(FramePassMeasure {
      info,
      pass_name,
      frame_index,
    }))) = self.time_statistics_pending.poll_next_unpin(cx)
    {
      self
        .collected
        .raw_entry_mut()
        .from_key(&pass_name)
        .or_insert_with(|| {
          (
            pass_name.clone(),
            PassRenderStatistics::new(self.max_history),
          )
        })
        .1
        .time
        .insert(
          info.duration_in_ms(self.time_unit_in_nanoseconds),
          frame_index,
        );
    }
  }

  pub fn clear_history(&mut self, max_history: usize) {
    self.collected.clear();
    self.max_history = max_history;
  }

  pub fn create_resolver(&mut self, frame_index: u64) -> FrameStaticInfoResolver {
    let (sub_pass_info_sender, sub_pass_info_receiver) = unbounded();
    let (sub_pass_time_sender, sub_pass_time_receiver) = unbounded();
    FrameStaticInfoResolver {
      sub_pass_info_sender,
      sub_pass_info_receiver,
      sub_pass_time_sender,
      sub_pass_time_receiver,
      statistic_output_sender: self.statistic_task_sender.clone(),
      time_output_sender: self.time_output_sender.clone(),
      frame_index,
      time_query_supported: self.time_query_supported,
      pipeline_query_supported: self.pipeline_query_supported,
    }
  }
}

pub struct FrameStaticInfoResolver {
  pub(crate) sub_pass_info_sender: UnboundedSender<FramePassMeasure<PipelineQueryResult>>,
  sub_pass_info_receiver: UnboundedReceiver<FramePassMeasure<PipelineQueryResult>>,

  pub(crate) sub_pass_time_sender: UnboundedSender<FramePassMeasure<TimeQuery>>,
  sub_pass_time_receiver: UnboundedReceiver<FramePassMeasure<TimeQuery>>,

  statistic_output_sender: UnboundedSender<StatisticTask>,
  time_output_sender: UnboundedSender<TimeTask>,
  frame_index: u64,

  pub time_query_supported: bool,
  pub pipeline_query_supported: bool,
}

impl FrameStaticInfoResolver {
  pub fn create_defer_logic(
    &self,
    pass: &mut GPURenderPass,
    gpu: &GPU,
  ) -> PassMeasurementDeferLogic {
    let pipeline_query = self
      .pipeline_query_supported
      .then(|| PipelineQuery::start(&gpu.device, pass));

    PassMeasurementDeferLogic {
      frame_index: self.frame_index,
      pipeline_query,
      pipeline_query_sender: self.sub_pass_info_sender.clone(),
      time_query_sender: self.sub_pass_time_sender.clone(),
    }
  }

  pub fn resolve(&mut self, gpu: &GPU, encoder: &mut GPUCommandEncoder) {
    noop_ctx!(cx);
    while let Poll::Ready(Some(FramePassMeasure {
      info,
      pass_name,
      frame_index,
    })) = self.sub_pass_info_receiver.poll_next_unpin(cx)
    {
      let f = info.read_back(&gpu.device, encoder);
      let f = f.map(move |r| {
        r.map(|info| FramePassMeasure {
          pass_name,
          info,
          frame_index,
        })
      });
      let f = Box::pin(f);
      self.statistic_output_sender.unbounded_send(f).ok();
    }

    while let Poll::Ready(Some(FramePassMeasure {
      info,
      pass_name,
      frame_index,
    })) = self.sub_pass_time_receiver.poll_next_unpin(cx)
    {
      let f = info.read_back(&gpu.device, encoder);
      let f = f.map(move |r| {
        r.map(|info| FramePassMeasure {
          pass_name,
          info,
          frame_index,
        })
      });
      let f = Box::pin(f);
      self.time_output_sender.unbounded_send(f).ok();
    }
  }
}

pub struct PassMeasurementDeferLogic {
  frame_index: u64,
  pipeline_query: Option<PipelineQuery>,
  pipeline_query_sender: UnboundedSender<FramePassMeasure<PipelineQueryResult>>,
  time_query_sender: UnboundedSender<FramePassMeasure<TimeQuery>>,
}

impl PassMeasurementDeferLogic {
  pub fn resolve_pipeline_stat(&mut self, pass: &mut GPURenderPass, desc: &RenderPassDescription) {
    if let Some(q) = self.pipeline_query.take() {
      let info = q.end(pass);
      self
        .pipeline_query_sender
        .unbounded_send(FramePassMeasure {
          pass_name: desc.name.clone(),
          info,
          frame_index: self.frame_index,
        })
        .ok();
    }
  }
  // split this to make sure the pass has already dropped
  pub fn resolve_pass_timing(
    &mut self,
    time_measuring: Option<TimeQuery>,
    desc: &RenderPassDescription,
  ) {
    if let Some(q) = time_measuring {
      self
        .time_query_sender
        .unbounded_send(FramePassMeasure {
          pass_name: desc.name.clone(),
          info: q,
          frame_index: self.frame_index,
        })
        .ok();
    }
  }
}

pub struct StatisticStore<T> {
  /// currently we only store the history but not do any analysis
  history: Vec<Option<(T, u64)>>,
  latest_resolved: Option<(T, u64)>,
}

impl<T: Clone> StatisticStore<T> {
  pub fn new(max_history: usize) -> Self {
    StatisticStore {
      history: vec![None; max_history],
      latest_resolved: None,
    }
  }
  pub fn clear(&mut self) {
    self.history.clear();
    self.latest_resolved = None;
  }

  pub fn get_latest(&self) -> Option<&(T, u64)> {
    self.latest_resolved.as_ref()
  }

  pub fn history_size(&self) -> usize {
    self.history.len()
  }

  pub fn history_max(&self) -> Option<&T>
  where
    T: PartialOrd,
  {
    self
      .history
      .iter()
      .filter_map(|v| v.as_ref().map(|v| &v.0))
      .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
  }

  pub fn iter_history_from_oldest_latest(&self) -> Option<impl Iterator<Item = Option<&T>>> {
    self.latest_resolved.as_ref().map(|(_, index)| {
      (0..self.history.len() as u64).map(move |offset| {
        if *index > offset {
          let access_index = (index - offset) % self.history.len() as u64;
          self.history[access_index as usize].as_ref().map(|v| &v.0)
        } else {
          None
        }
      })
    })
  }

  pub fn insert(&mut self, value: T, idx: u64) {
    let write_idx = idx as usize % self.history.len();
    self.history[write_idx] = Some((value.clone(), idx));
    if let Some(l) = &self.latest_resolved {
      if l.1 < idx {
        self.latest_resolved = Some((value, idx));
      }
    } else {
      self.latest_resolved = Some((value, idx));
    }
  }
}
