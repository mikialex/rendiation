use database::RawEntityHandle;

use crate::*;

pub struct MultiAccessGPUDataBuilderInit {
  pub max_possible_many_count: u32,
  pub max_possible_one_count: u32,
  pub init_many_count_capacity: u32,
  pub init_one_count_capacity: u32,
}

pub fn use_multi_access_gpu(
  cx: &mut QueryGPUHookCx,
  init: &MultiAccessGPUDataBuilderInit,
  source: UseResult<impl TriQueryLike<Key = RawEntityHandle, Value = RawEntityHandle>>,
  label: &str,
) -> Option<MultiAccessGPUData> {
  let (cx, allocator) = cx.use_sharable_plain_state(|| {
    GrowableRangeAllocator::new(init.max_possible_many_count, init.init_many_count_capacity)
  });

  let (cx, many_side_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc
      .allocate_readonly::<[u32]>(
        (init.init_many_count_capacity * 4) as u64,
        &gpu.device,
        Some(&format!("multi-access-many-side: {}", label)),
      )
      .with_direct_resize(gpu);
    Arc::new(RwLock::new(buffer))
  });

  let many_side_buffer_ = many_side_buffer.clone();
  let allocator = allocator.clone();
  let changes = source.map_only_spawn_stage_in_thread(
    cx,
    |source| !source.view_delta_ref().1.is_empty(),
    move |source| {
      let (multi_access, _, changes) = source.inv_view_view_delta();

      // collect all changed one.
      // for simplicity, we do full update for each "one"s data
      let mut dirtied_one = FastHashSet::default();
      for (_, change) in changes.iter_key_value() {
        match change {
          ValueChange::Delta(n, p) => {
            dirtied_one.insert(n);
            if let Some(p) = p {
              dirtied_one.insert(p);
            }
          }
          ValueChange::Remove(one) => {
            dirtied_one.insert(one);
          }
        }
      }

      // todo, avoid resize
      let mut buffers_to_write = RangeAllocateBufferCollector::default();
      let mut sizes = Vec::new();
      for one in &dirtied_one {
        if let Some(many_iter) = multi_access.access_multi(one) {
          let buffer = many_iter.map(|v| v.alloc_index()).collect::<Vec<_>>(); // todo, reuse allocation
          let buffer: &[u8] = cast_slice(&buffer);
          buffers_to_write.collect_direct(*one, buffer);
          sizes.push((*one, buffer.len() as u32 / 4));
        }
      }

      let allocation_changes = allocator
        .write()
        .update(dirtied_one.iter().copied(), sizes.iter().copied());

      let buffers_to_write = buffers_to_write.prepare(&allocation_changes, 4);

      let source_buffer = allocation_changes.resize_to.map(|new_size| {
        let mut gpu_buffer = many_side_buffer_.write();
        let buffer = gpu_buffer.abstract_gpu().get_gpu_buffer_view().unwrap();
        // here we do(request) resize at spawn stage to avoid resize again and again
        gpu_buffer.resize(new_size);
        buffer
      });

      Arc::new(RangeAllocateBufferUpdates {
        buffers_to_write,
        allocation_changes: BatchAllocateResultShared(Arc::new(allocation_changes), 4),
        source_buffer,
      })
    },
  );

  let (changes, changes_) = changes.fork();

  let updates = changes
    .map_only_spawn_stage_in_thread(
      cx,
      |changes| !changes.allocation_changes.has_change(),
      |changes| {
        let item_size = std::mem::size_of::<GPURangeInfo>();
        let change_count = changes.allocation_changes.0.change_count();
        let mut write_src =
          SparseBufferWritesSource::with_capacity(change_count * item_size, change_count);
        changes
          .allocation_changes
          .iter_update_or_insert()
          .for_each(|(id, value)| {
            let w_offset = item_size as u32 * id.alloc_index();
            let [offset, count] = value;

            let value = GPURangeInfo {
              start: offset / 4,
              len: count / 4,
              ..Default::default()
            };
            write_src.collect_write(bytes_of(&value), w_offset as u64);
          });
        Arc::new(write_src)
      },
    )
    .use_assure_result(cx);

  let changes_ = changes_.use_assure_result(cx);

  let (cx, one_side_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc.allocate_readonly::<[GPURangeInfo]>(
      (init.init_one_count_capacity * std::mem::size_of::<GPURangeInfo>() as u32) as u64,
      &gpu.device,
      Some(&format!("multi-access-one-side: {}", label)),
    );
    Arc::new(RwLock::new(buffer))
  });

  cx.when_render(|| {
    {
      let updates = updates.expect_resolve_stage();
      let buffer = one_side_buffer.write();
      // todo, this may failed if we support texture as storage buffer
      let target_buffer = buffer.get_gpu_buffer_view().unwrap();
      let mut encoder = cx.gpu.create_encoder(); // todo, reuse encoder and pass
      encoder.compute_pass_scoped(|mut pass| {
        updates.write(&cx.gpu.device, &mut pass, target_buffer);
      });
      cx.gpu.queue.submit_encoder(encoder);
    }

    {
      let buffer = many_side_buffer
        .write()
        .abstract_gpu()
        .get_gpu_buffer_view()
        .unwrap();
      let changes_ = changes_.expect_resolve_stage();
      changes_.write(cx.gpu, &buffer, 4);
      //
    }

    MultiAccessGPUData {
      meta: one_side_buffer.read().gpu().clone(),
      indices: many_side_buffer.read().gpu().clone(),
    }
  })
}

#[derive(Clone)]
pub struct MultiAccessGPUData {
  meta: AbstractReadonlyStorageBuffer<[GPURangeInfo]>,
  indices: AbstractReadonlyStorageBuffer<[u32]>,
}

impl MultiAccessGPUData {
  pub fn build(&self, cx: &mut ShaderBindGroupBuilder) -> MultiAccessGPUInvocation {
    MultiAccessGPUInvocation {
      meta: cx.bind_by(&self.meta),
      indices: cx.bind_by(&self.indices),
    }
  }

  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.meta);
    cx.bind(&self.indices);
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, PartialEq, Debug)]
pub struct GPURangeInfo {
  pub start: u32,
  pub len: u32,
}

impl Default for GPURangeInfo {
  fn default() -> Self {
    Self {
      start: u32::MAX, // we use max as empty hint
      len: Default::default(),
      ..Zeroable::zeroed()
    }
  }
}

#[derive(Clone)]
pub struct MultiAccessGPUInvocation {
  pub meta: ShaderReadonlyPtrOf<[GPURangeInfo]>,
  pub indices: ShaderReadonlyPtrOf<[u32]>,
}

impl MultiAccessGPUInvocation {
  pub fn iter_refed_many_of(&self, one: Node<u32>) -> impl ShaderIterator<Item = Node<u32>> {
    MultiAccessGPUIter {
      indices: self.indices.clone(),
      meta: self.meta.index(one).load().expand(),
      cursor: val(0_u32).make_local_var(),
    }
  }
  pub fn get_n_th(&self, one: Node<u32>, n: Node<u32>) -> Node<u32> {
    let offset = self.meta.index(one).load().expand().start;
    self.indices.index(offset + n).load()
  }
}

struct MultiAccessGPUIter {
  indices: ShaderReadonlyPtrOf<[u32]>,
  meta: ENode<GPURangeInfo>,
  cursor: ShaderPtrOf<u32>,
}

impl ShaderIterator for MultiAccessGPUIter {
  type Item = Node<u32>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(self.meta.len);
    let data = self
      .indices
      .index(current_next.min(self.meta.len - val(1)) + self.meta.start)
      .load();
    (has_next, data)
  }
}
