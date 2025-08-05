use rendiation_texture_core::SizeWithDepth;

use self::{
  growable::{GrowablePacker, PackResultRelocation},
  pack_impl::etagere_wrap::EtagerePacker,
};
use super::*;

pub struct RemappedGrowablePacker {
  max_size: SizeWithDepth,
  packer: GrowablePacker<MultiLayerTexturePacker<EtagerePacker>>,
  // todo, i think this is not necessary if the packer lib not generate id
  mapping: FastHashMap<PackId, u32>,
  rev_mapping: FastHashMap<u32, PackId>,
}

impl RemappedGrowablePacker {
  pub fn new(config: MultiLayerTexturePackerConfig) -> Self {
    Self {
      packer: GrowablePacker::new(config.init_size),
      max_size: config.max_size,
      mapping: Default::default(),
      rev_mapping: Default::default(),
    }
  }

  pub fn process(
    &mut self,
    iter_removed: impl Iterator<Item = u32>,
    iter_changed_or_insert: impl Iterator<Item = (u32, Size)>,
    notify_resize: impl Fn(SizeWithDepth),
    notify_change: impl Fn(u32, ValueChange<PackResult2dWithDepth>),
  ) {
    let mapping = &mut self.mapping;
    let rev_mapping = &mut self.rev_mapping;
    let packer = &mut self.packer;

    let mut grow = |config: SizeWithDepth| {
      let max = self.max_size;
      let width_capacity = max.size.width_usize() - config.size.width_usize();
      let height_capacity = max.size.height_usize() - config.size.height_usize();
      let depth_capacity = u32::from(max.depth) - u32::from(config.depth);

      if depth_capacity == 0 && height_capacity == 0 && width_capacity == 0 {
        if ENABLE_DEBUG_LOG {
          println!("grow failed, current_size: {config:?}, max_size: {max:?}");
        }

        return None;
      }

      let target_config = if height_capacity == 0 && width_capacity == 0 {
        // when we only have depth space available, increase depth only one step each grow
        SizeWithDepth {
          depth: NonZeroU32::new(u32::from(config.depth) + 1).unwrap(),
          size: config.size,
        }
      } else {
        let width_target = (config.size.width_usize() * 2).min(max.size.width_usize());
        let height_target = (config.size.width_usize() * 2).min(max.size.width_usize());
        SizeWithDepth {
          depth: config.depth,
          size: Size::from_usize_pair_min_one((width_target, height_target)),
        }
      };

      if ENABLE_DEBUG_LOG {
        println!("grow success, current_size: {target_config:?}, max_size: {max:?}");
      }
      notify_resize(target_config);

      Some(target_config)
    };

    // do all remove first
    for id in iter_removed {
      let pack_id = rev_mapping.remove(&id).unwrap();
      mapping.remove(&pack_id);
      let previous = packer.unpack(pack_id).unwrap();
      let delta = ValueChange::Remove(previous);
      notify_change(id, delta);
    }

    for (id, size) in iter_changed_or_insert {
      if let Some(pack_id) = rev_mapping.remove(&id) {
        mapping.remove(&pack_id);
        let previous = packer.unpack(pack_id).unwrap();
        let delta = ValueChange::Remove(previous);
        notify_change(id, delta);
      }

      let mut staging_mapping = FastHashMap::default();
      let mut relocate = |relocation: PackResultRelocation<PackResult2dWithDepth>| {
        let idx = staging_mapping
          .remove(&relocation.previous.id)
          .or_else(|| mapping.remove(&relocation.previous.id).unwrap().into())
          .unwrap();

        let previous = relocation.previous.result;
        notify_change(idx, ValueChange::Remove(previous));

        if let Some(overridden) = mapping.insert(relocation.new.id, idx) {
          staging_mapping.insert(relocation.new.id, overridden);
        }
        let current = relocation.new.result;
        notify_change(idx, ValueChange::Delta(current, None));

        rev_mapping.insert(idx, relocation.new.id);
      };

      let pack_result = packer.pack_and_check_grow(size, &mut grow, &mut relocate);

      if let Ok(pack_result) = pack_result {
        rev_mapping.insert(id, pack_result.id);
        mapping.insert(pack_result.id, id);
        let delta = ValueChange::Delta(pack_result.result, None);

        notify_change(id, delta);
      } else {
        println!("warning, texture allocation failed for {id}, try increase the max size")
      }
    }
  }
}

pub fn reactive_pack_2d_to_3d(
  mut config: MultiLayerTexturePackerConfig,
  size: BoxedDynReactiveQuery<u32, Size>,
) -> (
  impl ReactiveQuery<Key = u32, Value = PackResult2dWithDepth>,
  impl Stream<Item = SizeWithDepth> + Unpin,
) {
  config.make_sure_valid();

  let (size_sender, size_rev) = single_value_channel();
  size_sender.update(config.init_size).unwrap();

  let packer = Packer {
    packer: Arc::new(RwLock::new(RemappedGrowablePacker::new(config))),
    size_source: size,
    all_size_sender: Arc::new(size_sender),
  };

  (packer, size_rev)
}

struct Packer {
  size_source: BoxedDynReactiveQuery<u32, Size>,
  packer: Arc<RwLock<RemappedGrowablePacker>>,
  all_size_sender: Arc<SingleSender<SizeWithDepth>>,
}

#[derive(Clone)]
struct PackerCurrentView(LockReadGuardHolder<RemappedGrowablePacker>);

impl Query for PackerCurrentView {
  type Key = u32;
  type Value = PackResult2dWithDepth;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, PackResult2dWithDepth)> + '_ {
    self
      .0
      .rev_mapping
      .iter()
      .map(|(k, v)| (*k, self.0.packer.current_states().1.get(v).unwrap().1))
  }

  fn access(&self, key: &u32) -> Option<PackResult2dWithDepth> {
    let pack_id = self.0.rev_mapping.get(key)?;
    let result = self.0.packer.current_states().1.get(pack_id)?.1;
    Some(result)
  }
}

impl ReactiveQuery for Packer {
  type Key = u32;
  type Value = PackResult2dWithDepth;

  type Compute = PackerCompute<BoxedDynQueryCompute<u32, Size>>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    PackerCompute {
      size_source: self.size_source.describe_dyn(cx),
      packer: self.packer.clone(),
      all_size_sender: self.all_size_sender.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    // consider trigger packer shrink logic here?
    self.size_source.request(request);
  }
}

struct PackerCompute<T> {
  size_source: T,
  packer: Arc<RwLock<RemappedGrowablePacker>>,
  all_size_sender: Arc<SingleSender<SizeWithDepth>>,
}

impl<T: AsyncQueryCompute<Key = u32, Value = Size>> AsyncQueryCompute for PackerCompute<T> {
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let packer = self.packer.clone();
    let all_size_sender = self.all_size_sender.clone();
    let size_source = self.size_source.create_task(cx);
    cx.then_spawn_compute(size_source, move |size_source| PackerCompute {
      size_source,
      packer,
      all_size_sender,
    })
    .into_boxed_future()
  }
}

const ENABLE_DEBUG_LOG: bool = false;

impl<T: QueryCompute<Key = u32, Value = Size>> QueryCompute for PackerCompute<T> {
  type Key = u32;
  type Value = PackResult2dWithDepth;

  type Changes = Option<FastHashMap<u32, ValueChange<PackResult2dWithDepth>>>;
  type View = PackerCurrentView;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.size_source.resolve(cx);
    cx.keep_view_alive(v);
    let (sender, mut rev) = collective_channel::<u32, PackResult2dWithDepth>();

    {
      unsafe {
        sender.lock();
        let packer = &mut self.packer.write();

        let removes = d
          .iter_key_value()
          .filter_map(|(k, v)| v.is_removed().then_some(k));

        let changes = d
          .iter_key_value()
          .filter_map(|(k, v)| v.new_value().map(|v| (k, *v)));

        packer.process(
          removes,
          changes,
          |new_size| {
            self.all_size_sender.update(new_size).ok();
          },
          |idx, change| sender.send(idx, change),
        );

        sender.unlock();
      }
    }

    let v = PackerCurrentView(self.packer.make_read_holder());

    noop_ctx!(cx);
    let mut d = None;
    if let Poll::Ready(Some(r)) = rev.poll_next_unpin(cx) {
      d = Some(r);
    }

    (d, v)
  }
}
