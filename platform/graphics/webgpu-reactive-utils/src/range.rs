use std::ops::DerefMut;

use parking_lot::RwLock;

use crate::*;

pub type UntypedPool = Arc<RwLock<StorageBufferRangeAllocatePool<u32>>>;

pub struct ReactiveRangeAllocatePool<T: ReactiveQuery> {
  buffer: UntypedPool,
  record: Arc<RwLock<FastHashMap<T::Key, u32>>>,
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

impl<T: ReactiveQuery<Value = (Arc<Vec<u8>>, Option<GPUBufferViewRange>)>> ReactiveQuery
  for ReactiveRangeAllocatePool<T>
{
  type Key = T::Key;
  type Value = u32;

  type Changes = impl Query<Key = T::Key, Value = ValueChange<u32>>;
  type View = impl Query<Key = T::Key, Value = u32>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let mut record = self.record.write();
    let mut buffer = self.buffer.write();
    let mut rev = self.rev_map.write();
    let (d, _) = self.upstream.poll_changes(cx);

    let mut mutations = FastHashMap::<T::Key, ValueChange<u32>>::default();
    let mut mutator = QueryMutationCollector {
      delta: &mut mutations,
      target: record.deref_mut(),
    };

    // do all deallocation first to avoid peak memory consumption
    for (k, change) in d.iter_key_value() {
      if let ValueChange::Remove(_) = change {
        let offset = mutator.remove(k).unwrap();
        rev.remove(&offset);
        buffer.deallocate(offset);
      }
    }

    for (k, change) in d.iter_key_value() {
      if let ValueChange::Delta(new, old) = change {
        // always deallocate first to minimize peak memory usage
        if old.is_some() {
          let offset = mutator.remove(k.clone()).unwrap();
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

        let count = data_to_write.len();
        let offset = buffer
          .allocate_range(count as u32, &mut |relocation| {
            let id = rev.remove(&relocation.previous_offset).unwrap();
            mutator.remove(id.clone());
            rev.insert(relocation.new_offset, id.clone());
            mutator.set_value(id, relocation.new_offset);
          })
          .unwrap();
        let gpu_buffer = buffer.raw_gpu();
        self
          .gpu
          .queue
          .write_buffer(gpu_buffer.resource.gpu(), offset as u64, data_to_write);

        mutator.set_value(k.clone(), offset);
        rev.insert(offset, k);
      }
    }

    drop(record);
    (mutations, self.record.make_read_holder())
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
  }
}
