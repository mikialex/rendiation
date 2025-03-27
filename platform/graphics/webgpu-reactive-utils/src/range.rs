use std::ops::DerefMut;

use parking_lot::RwLock;

use crate::*;

pub type UntypedPool = Arc<RwLock<StorageBufferRangeAllocatePool<u32>>>;

pub struct ReactiveRangeAllocatePool<T: ReactiveQuery> {
  buffer: UntypedPool,
  record: Arc<RwLock<FastHashMap<T::Key, (u32, u32)>>>,
  rev_map: Arc<RwLock<FastHashMap<u32, T::Key>>>,
  gpu: GPU,
  upstream: T,
}

impl<T: ReactiveQuery> ReactiveRangeAllocatePool<T> {
  pub fn new(buffer: &UntypedPool, upstream: T, gpu: &GPU) -> Self {
    Self {
      buffer: buffer.clone(),
      record: Default::default(),
      rev_map: Default::default(),
      gpu: gpu.clone(),
      upstream,
    }
  }
}

pub struct ReactiveRangeAllocatePoolCompute<T: QueryCompute> {
  buffer: UntypedPool,
  record: Arc<RwLock<FastHashMap<T::Key, (u32, u32)>>>,
  rev_map: Arc<RwLock<FastHashMap<u32, T::Key>>>,
  gpu: GPU,
  upstream: T,
}

impl<T: AsyncQueryCompute<Value = (Arc<Vec<u8>>, Option<GPUBufferViewRange>)>> AsyncQueryCompute
  for ReactiveRangeAllocatePoolCompute<T>
{
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let buffer = self.buffer.clone();
    let record = self.record.clone();
    let gpu = self.gpu.clone();
    let rev_map = self.rev_map.clone();
    let upstream = self.upstream.create_task(cx);
    cx.then_spawn(upstream, |upstream, cx| {
      ReactiveRangeAllocatePoolCompute {
        buffer,
        record,
        rev_map,
        gpu,
        upstream,
      }
      .resolve(cx)
    })
  }
}

impl<T: QueryCompute<Value = (Arc<Vec<u8>>, Option<GPUBufferViewRange>)>> QueryCompute
  for ReactiveRangeAllocatePoolCompute<T>
{
  type Key = T::Key;
  type Value = (u32, u32); // offset count(in u32)

  type Changes = Arc<FastHashMap<T::Key, ValueChange<(u32, u32)>>>;
  type View = LockReadGuardHolder<FastHashMap<T::Key, (u32, u32)>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let mut record = self.record.write();
    let mut buffer = self.buffer.write();
    let mut rev = self.rev_map.write();
    let (d, v) = self.upstream.resolve(cx);
    cx.keep_view_alive(v);

    let mut mutations = FastHashMap::<T::Key, ValueChange<(u32, u32)>>::default();
    let mut mutator = QueryMutationCollector {
      delta: &mut mutations,
      target: record.deref_mut(),
    };

    // do all deallocation first to avoid peak memory consumption
    for (k, change) in d.iter_key_value() {
      if let ValueChange::Remove(_) = change {
        let (offset, _) = mutator.remove(k).unwrap();
        rev.remove(&offset);
        buffer.deallocate(offset);
      }
    }

    for (k, change) in d.iter_key_value() {
      if let ValueChange::Delta(new, old) = change {
        // always deallocate first to minimize peak memory usage
        if old.is_some() {
          let (offset, _) = mutator.remove(k.clone()).unwrap();
          buffer.deallocate(offset);
          rev.remove(&offset);
        }
        let data_to_write = if let Some(range) = new.1 {
          let start = range.offset as usize;
          let end = if let Some(size) = range.size {
            start + u64::from(size) as usize
          } else {
            new.0.len()
          };
          new.0.get(start..end).unwrap()
        } else {
          new.0.as_slice()
        };

        let mut override_offsets: FastHashMap<u32, T::Key> = FastHashMap::default();
        let count = data_to_write.len();
        assert_eq!(count % 4, 0);
        let count_u32 = (count / 4) as u32;
        let offset = buffer
          .allocate_range(count_u32, &mut |relocation| {
            let id = override_offsets
              .remove(&relocation.previous_offset)
              .unwrap_or_else(|| rev.remove(&relocation.previous_offset).unwrap());

            mutator.remove(id.clone());
            if let Some(overridden) = rev.insert(relocation.new_offset, id.clone()) {
              override_offsets.insert(relocation.new_offset, overridden);
            }
            mutator.set_value(id, (relocation.new_offset, count_u32));
          })
          .unwrap();
        assert!(override_offsets.is_empty());

        let gpu_buffer = buffer.raw_gpu();
        self.gpu.queue.write_buffer(
          gpu_buffer.resource.gpu(),
          (offset * 4) as u64,
          data_to_write,
        );

        mutator.set_value(k.clone(), (offset, count_u32));
        rev.insert(offset, k);
      }
    }

    drop(record);
    (Arc::new(mutations), self.record.make_read_holder())
  }
}

impl<T: ReactiveQuery<Value = (Arc<Vec<u8>>, Option<GPUBufferViewRange>)>> ReactiveQuery
  for ReactiveRangeAllocatePool<T>
{
  type Key = T::Key;
  type Value = (u32, u32); // offset count(in u32)

  type Compute = ReactiveRangeAllocatePoolCompute<T::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    ReactiveRangeAllocatePoolCompute {
      buffer: self.buffer.clone(),
      record: self.record.clone(),
      rev_map: self.rev_map.clone(),
      gpu: self.gpu.clone(),
      upstream: self.upstream.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
  }
}
