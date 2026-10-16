use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures::channel::mpsc::*;
use futures::{FutureExt, StreamExt};

use crate::*;

/// The persistent pass statistics collector.
///
/// For each frame, a [FramePassStatisticsRecorder] is created to record the queries of all passes
/// in that frame, the results are read back asynchronously and collected by [FramePassStatistics::poll].
///
/// The passes with the same name in one frame are merged(summed) together.
pub struct FramePassStatistics {
  result_sender: UnboundedSender<FrameMeasureTask>,
  pending: Pin<Box<dyn futures::Stream<Item = FramePassMeasureResult>>>,
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
    let (result_sender, receiver) = unbounded();
    // each frame produces only one task, the limit is the max frame in flight
    let pending = receiver.buffer_unordered(16).boxed();

    let features = gpu.info.supported_features;
    Self {
      time_unit_in_nanoseconds: gpu.queue.get_timestamp_period(),
      result_sender,
      pending,
      collected: FastHashMap::default(),
      max_history,
      time_query_supported: features.contains(Features::TIMESTAMP_QUERY),
      pipeline_query_supported: features.contains(Features::PIPELINE_STATISTICS_QUERY),
    }
  }

  pub fn poll(&mut self, cx: &mut Context) {
    while let Poll::Ready(Some(result)) = self.pending.poll_next_unpin(cx) {
      self.collect(result);
    }
  }

  fn collect(&mut self, result: FramePassMeasureResult) {
    let mut merged = FastHashMap::<&str, (Option<DeviceDrawStatistics>, Option<f64>)>::default();
    for (i, name) in result.pass_names.iter().enumerate() {
      let (pipeline, time) = merged.entry(name.as_str()).or_default();

      if let Some(info) = result.pipeline.as_ref().and_then(|v| v.get(i)) {
        *pipeline = Some(pipeline.map_or(*info, |p| p + *info));
      }

      if let Some(pair) = result.timestamps.as_ref().and_then(|v| v.get(i)) {
        let duration = pair.duration_in_ms(self.time_unit_in_nanoseconds);
        if !duration.is_nan() {
          *time = Some(time.unwrap_or(0.) + duration);
        }
      }
    }

    for (name, (pipeline, time)) in merged {
      let max_history = self.max_history;
      let (_, statistics) = self
        .collected
        .raw_entry_mut()
        .from_key(name)
        .or_insert_with(|| (name.to_string(), PassRenderStatistics::new(max_history)));

      if let Some(pipeline) = pipeline {
        statistics.pipeline.insert(pipeline, result.frame_index);
      }
      if let Some(time) = time {
        statistics.time.insert(time, result.frame_index);
      }
    }
  }

  pub fn clear_history(&mut self, max_history: usize) {
    self.collected.clear();
    self.max_history = max_history;
  }

  pub fn create_frame_recorder(&self, frame_index: u64) -> FramePassStatisticsRecorder {
    FramePassStatisticsRecorder {
      frame_index,
      pass_names: Default::default(),
      timestamp: self
        .time_query_supported
        .then(|| ChunkedQuerySet::new(QueryType::Timestamp, 2)),
      pipeline: self.pipeline_query_supported.then(|| {
        ChunkedQuerySet::new(
          QueryType::PipelineStatistics(PipelineStatisticsTypes::all()),
          1,
        )
      }),
      result_sender: self.result_sender.clone(),
    }
  }
}

type FrameMeasureTask = Pin<Box<dyn Future<Output = FramePassMeasureResult> + Send>>;

struct FramePassMeasureResult {
  frame_index: u64,
  pass_names: Vec<String>,
  /// indexed by the pass index
  timestamps: Option<Vec<TimestampPair>>,
  /// indexed by the pass index
  pipeline: Option<Vec<DeviceDrawStatistics>>,
}

/// Record the queries of all passes in one frame.
///
/// The queries are written into the query sets shared by all passes in this frame, and resolved
/// and read back together at the end of the frame, see [FramePassStatisticsRecorder::resolve].
pub struct FramePassStatisticsRecorder {
  frame_index: u64,
  pass_names: Vec<String>,
  timestamp: Option<ChunkedQuerySet>,
  pipeline: Option<ChunkedQuerySet>,
  result_sender: UnboundedSender<FrameMeasureTask>,
}

impl FramePassStatisticsRecorder {
  /// begin the render pass with the queries enabled, return the pass and if the pipeline
  /// statistics query is started(which should be ended before the pass ends).
  pub fn begin_pass(
    &mut self,
    desc: &RenderPassDescription,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) -> (GPURenderPass, bool) {
    let pass_index = self.pass_names.len() as u32;
    self.pass_names.push(desc.name.clone());

    let timestamp_writes = self.timestamp.as_mut().map(|q| {
      let (query_set, index) = q.locate(device, pass_index);
      raw_gpu::RenderPassTimestampWrites {
        query_set,
        beginning_of_pass_write_index: Some(index),
        end_of_pass_write_index: Some(index + 1),
      }
    });

    let mut pass = encoder.begin_render_pass(desc, timestamp_writes);

    let pipeline_query_active = if let Some(q) = &mut self.pipeline {
      let (query_set, index) = q.locate(device, pass_index);
      pass.begin_pipeline_statistics_query(query_set, index);
      true
    } else {
      false
    };

    (pass, pipeline_query_active)
  }

  /// resolve and read back the queries of all recorded passes, should be called after all passes
  /// are ended in the given encoder.
  pub fn resolve(&mut self, device: &GPUDevice, encoder: &mut GPUCommandEncoder) {
    let pass_count = self.pass_names.len() as u32;
    if pass_count == 0 {
      return;
    }

    let timestamps = self
      .timestamp
      .as_ref()
      .map(|q| q.resolve_and_read_back::<TimestampPair>(pass_count, device, encoder));
    let pipeline = self
      .pipeline
      .as_ref()
      .map(|q| q.resolve_and_read_back::<DeviceDrawStatistics>(pass_count, device, encoder));

    let pass_names = std::mem::take(&mut self.pass_names);
    let frame_index = self.frame_index;

    let task = async move {
      let timestamps = match timestamps {
        Some(f) => f.await,
        None => None,
      };
      let pipeline = match pipeline {
        Some(f) => f.await,
        None => None,
      };
      FramePassMeasureResult {
        frame_index,
        pass_names,
        timestamps,
        pipeline,
      }
    };
    self.result_sender.unbounded_send(Box::pin(task)).ok();
  }
}

/// the pass count in one query set, the query set is allocated lazily by chunks because the pass
/// count is unknown before the frame ends.
const PASS_COUNT_PER_CHUNK: u32 = 64;

struct ChunkedQuerySet {
  ty: QueryType,
  query_count_per_pass: u32,
  chunks: Vec<raw_gpu::QuerySet>,
}

impl ChunkedQuerySet {
  fn new(ty: QueryType, query_count_per_pass: u32) -> Self {
    Self {
      ty,
      query_count_per_pass,
      chunks: Default::default(),
    }
  }

  /// return the query set and the first query index of the given pass
  fn locate(&mut self, device: &GPUDevice, pass_index: u32) -> (&raw_gpu::QuerySet, u32) {
    let chunk_index = (pass_index / PASS_COUNT_PER_CHUNK) as usize;
    while self.chunks.len() <= chunk_index {
      let query_set = device.create_query_set(&QuerySetDescriptor {
        label: "frame pass statistics".into(),
        ty: self.ty,
        count: PASS_COUNT_PER_CHUNK * self.query_count_per_pass,
      });
      self.chunks.push(query_set);
    }
    let index = pass_index % PASS_COUNT_PER_CHUNK * self.query_count_per_pass;
    (&self.chunks[chunk_index], index)
  }

  /// T is the resolved result of one pass
  fn resolve_and_read_back<T: Pod>(
    &self,
    pass_count: u32,
    device: &GPUDevice,
    encoder: &mut GPUCommandEncoder,
  ) -> impl Future<Output = Option<Vec<T>>> + Send + use<T> {
    let pass_byte_size = std::mem::size_of::<T>() as u64;
    let chunk_byte_size = PASS_COUNT_PER_CHUNK as u64 * pass_byte_size;
    // the resolve target offset of each chunk must be aligned
    assert!(chunk_byte_size.is_multiple_of(raw_gpu::QUERY_RESOLVE_BUFFER_ALIGNMENT));

    let usage = BufferUsages::COPY_SRC | BufferUsages::QUERY_RESOLVE;
    let byte_size = self.chunks.len() as u64 * chunk_byte_size;
    let resolved = create_gpu_buffer_zeroed(byte_size, usage, device).create_default_view();

    for (i, query_set) in self.chunks.iter().enumerate() {
      let chunk_pass_start = i as u32 * PASS_COUNT_PER_CHUNK;
      let chunk_pass_count = (pass_count - chunk_pass_start).min(PASS_COUNT_PER_CHUNK);
      encoder.resolve_query_set(
        query_set,
        0..chunk_pass_count * self.query_count_per_pass,
        resolved.resource.gpu(),
        i as u64 * chunk_byte_size,
      );
    }

    let read_byte_size = (pass_count as u64 * pass_byte_size) as usize;
    encoder.read_buffer(device, &resolved).map(move |r| {
      r.ok()
        .map(|r| bytemuck::pod_collect_to_vec(&r.read_raw()[..read_byte_size]))
    })
  }
}

pub struct StatisticStore<T> {
  /// a ring buffer indexed by the frame index
  history: Vec<Option<(T, u64)>>,
  latest_resolved: Option<(T, u64)>,
}

impl<T: Clone> StatisticStore<T> {
  pub fn new(max_history: usize) -> Self {
    StatisticStore {
      history: vec![None; max_history.max(1)],
      latest_resolved: None,
    }
  }
  pub fn clear(&mut self) {
    self.history.fill(None);
    self.latest_resolved = None;
  }

  pub fn get_latest(&self) -> Option<&(T, u64)> {
    self.latest_resolved.as_ref()
  }

  pub fn history_size(&self) -> usize {
    self.history.len()
  }

  fn iter_valid_history(&self) -> impl Iterator<Item = &T> + Clone {
    self.history.iter().filter_map(|v| v.as_ref().map(|v| &v.0))
  }

  pub fn history_max(&self) -> Option<&T>
  where
    T: PartialOrd,
  {
    self
      .iter_valid_history()
      .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
  }

  /// iterate the history of the latest [StatisticStore::history_size] frames, from the oldest to
  /// the latest. None means the frame has no record.
  pub fn iter_history_from_oldest_latest(&self) -> Option<impl Iterator<Item = Option<&T>>> {
    let len = self.history.len() as u64;
    self.latest_resolved.as_ref().map(move |(_, latest)| {
      let latest = *latest;
      (0..len).rev().map(move |offset| {
        let frame_index = latest.checked_sub(offset)?;
        let (value, recorded_index) = self.history[(frame_index % len) as usize].as_ref()?;
        // the slot may be written by the frame before the history window
        (*recorded_index == frame_index).then_some(value)
      })
    })
  }

  pub fn insert(&mut self, value: T, idx: u64) {
    let write_idx = idx as usize % self.history.len();
    self.history[write_idx] = Some((value.clone(), idx));
    if self.latest_resolved.as_ref().is_none_or(|l| l.1 < idx) {
      self.latest_resolved = Some((value, idx));
    }
  }
}

impl StatisticStore<f32> {
  /// return None if the history is empty
  pub fn history_average(&self) -> Option<f32> {
    let iter = self.iter_valid_history();
    let count = iter.clone().count();
    (count > 0).then(|| iter.sum::<f32>() / count as f32)
  }
}

#[cfg(test)]
mod test {
  use std::time::{Duration, Instant};

  use super::*;

  #[test]
  fn test_frame_pass_statistics() {
    let gpu = pollster::block_on(GPU::new(Default::default())).unwrap();
    let mut statistics = FramePassStatistics::new(4, &gpu);
    let pool = init_attachment_pool(&gpu);
    let pass_info_pool = PassInfoPool::default();
    let mut memory = FunctionMemory::default();

    // cross the chunk boundary, and the passes with same name should be merged
    let pass_count = PASS_COUNT_PER_CHUNK + 6;
    let frame_index = 1;
    {
      let recorder = statistics.create_frame_recorder(frame_index);
      let size = Size::from_u32_pair_min_one((4, 4));
      let mut ctx = FrameCtx::new(
        &gpu,
        &mut memory,
        size,
        &pool,
        &pass_info_pool,
        Some(recorder),
      );
      let target = attachment().request(&ctx);
      for i in 0..pass_count {
        let name = if i % 2 == 0 { "even" } else { "odd" };
        let _ = pass(name)
          .with_color(&target, store_full_frame())
          .render_ctx(&mut ctx);
      }
    }

    let any_query_supported =
      statistics.time_query_supported || statistics.pipeline_query_supported;
    if !any_query_supported {
      return;
    }

    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    let start = Instant::now();
    while statistics.collected.len() < 2 && start.elapsed() < Duration::from_secs(5) {
      statistics.poll(&mut cx);
      std::thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(statistics.collected.len(), 2);
    for name in ["even", "odd"] {
      let s = &statistics.collected[name];
      if statistics.time_query_supported {
        let (time, index) = s.time.get_latest().unwrap();
        assert_eq!(*index, frame_index);
        assert!(*time >= 0.);
      }
      if statistics.pipeline_query_supported {
        let (_, index) = s.pipeline.get_latest().unwrap();
        assert_eq!(*index, frame_index);
      }
    }
  }

  #[test]
  fn test_statistic_store() {
    let mut store = StatisticStore::<f32>::new(4);
    assert_eq!(store.history_average(), None);

    store.insert(1., 1);
    store.insert(3., 3);
    // the slot of frame 2 is not written, frame 0 is out of window
    let history: Vec<_> = store
      .iter_history_from_oldest_latest()
      .unwrap()
      .map(|v| v.copied())
      .collect();
    assert_eq!(history, vec![None, Some(1.), None, Some(3.)]);

    // overwrite the slot of frame 1, frame 1 should be treated as no record
    store.insert(5., 5);
    let history: Vec<_> = store
      .iter_history_from_oldest_latest()
      .unwrap()
      .map(|v| v.copied())
      .collect();
    assert_eq!(history, vec![None, Some(3.), None, Some(5.)]);
    assert_eq!(store.history_average(), Some(4.));

    store.clear();
    store.insert(2., 6);
    assert_eq!(store.get_latest(), Some(&(2., 6)));
  }
}
