use std::ops::DerefMut;

use parking_lot::RwLock;

use crate::*;

pub type UntypedU32Pool = Arc<RwLock<StorageBufferRangeAllocatePool<u32>>>;

#[derive(Clone)]
pub struct ReactiveRangeAllocatePool<K> {
  buffer: UntypedU32Pool,
  record: Arc<RwLock<FastHashMap<K, [u32; 2]>>>,
  rev_map: Arc<RwLock<FastHashMap<u32, K>>>,
}

impl<K> ReactiveRangeAllocatePool<K> {
  pub fn new(buffer: &UntypedU32Pool) -> Self {
    Self {
      buffer: buffer.clone(),
      record: Default::default(),
      rev_map: Default::default(),
    }
  }
}

impl<K: CKey> ReactiveRangeAllocatePool<K> {
  pub fn update<'a>(
    &self,
    removed_and_changed_keys: impl Iterator<Item = K>,
    changed_keys: impl Iterator<Item = (K, &'a [u8])>,
    gpu: &GPU,
  ) -> BoxedDynDualQuery<K, [u32; 2]> {
    let mut record = self.record.write();
    let mut buffer = self.buffer.write();
    let mut rev = self.rev_map.write();

    let mut mutations = FastHashMap::<K, ValueChange<[u32; 2]>>::default();
    let mut mutator = QueryMutationCollector {
      delta: &mut mutations,
      target: record.deref_mut(),
    };

    // the changed key also need deallocate first
    for k in removed_and_changed_keys {
      if let Some([offset, _]) = mutator.remove(k) {
        rev.remove(&offset);
        buffer.deallocate(offset);
      }
    }

    let mut override_offsets: FastHashMap<u32, K> = FastHashMap::default();
    for (k, data_to_write) in changed_keys {
      let count = data_to_write.len();
      assert_eq!(count % 4, 0);
      let count_u32 = (count / 4) as u32;

      let offset = buffer
        .allocate_range(count_u32, &mut |relocation| {
          let id = override_offsets
            .remove(&relocation.previous_offset)
            .unwrap_or_else(|| rev.remove(&relocation.previous_offset).unwrap());

          let [_, count] = mutator.remove(id.clone()).unwrap();
          if let Some(overridden) = rev.insert(relocation.new_offset, id.clone()) {
            override_offsets.insert(relocation.new_offset, overridden);
          }
          mutator.set_value(id, [relocation.new_offset, count]);
        })
        .unwrap();
      assert!(override_offsets.is_empty());

      let gpu_buffer = buffer.raw_gpu();
      gpu.queue.write_buffer(
        gpu_buffer.resource.gpu(),
        (offset * 4) as u64,
        data_to_write,
      );

      mutator.set_value(k.clone(), [offset, count_u32]);
      rev.insert(offset, k);
    }

    drop(record);

    DualQuery {
      delta: Arc::new(mutations),
      view: self.record.make_read_holder(),
    }
    .into_boxed()
  }
}
