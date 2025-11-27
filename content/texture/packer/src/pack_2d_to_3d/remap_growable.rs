use std::hash::Hash;

use rendiation_texture_core::SizeWithDepth;

use self::{
  growable::{GrowablePacker, PackResultRelocation},
  pack_impl::etagere_wrap::EtagerePacker,
};
use super::*;

const ENABLE_DEBUG_LOG: bool = true;

pub struct RemappedGrowablePacker<K> {
  max_size: SizeWithDepth,
  packer: GrowablePacker<MultiLayerTexturePacker<EtagerePacker>>,
  // todo, i think this is not necessary if the packer lib not generate id
  mapping: FastHashMap<PackId, K>,
  rev_mapping: FastHashMap<K, Option<PackId>>, // none means allocation failed
}

impl<K: Copy + Eq + Hash + std::fmt::Debug> RemappedGrowablePacker<K> {
  pub fn new(config: MultiLayerTexturePackerConfig) -> Self {
    Self {
      packer: GrowablePacker::new(config.init_size),
      max_size: config.max_size,
      mapping: Default::default(),
      rev_mapping: Default::default(),
    }
  }

  pub fn packed_count(&self) -> usize {
    self.mapping.len()
  }

  pub fn is_empty(&self) -> bool {
    self.mapping.is_empty()
  }

  pub fn current_size(&self) -> SizeWithDepth {
    *self.packer.current_states().0
  }

  pub fn iter_key_value(&self) -> impl Iterator<Item = (K, Option<PackResult2dWithDepth>)> + '_ {
    self.rev_mapping.iter().map(|(k, v)| {
      (
        *k,
        v.map(|v| self.packer.current_states().1.get(&v).unwrap().1),
      )
    })
  }

  pub fn access(&self, key: &K) -> Option<Option<PackResult2dWithDepth>> {
    if let Some(pack_id) = self.rev_mapping.get(key)? {
      let result = self.packer.current_states().1.get(pack_id)?.1;
      Some(Some(result))
    } else {
      Some(None)
    }
  }

  pub fn process(
    &mut self,
    iter_removed: impl Iterator<Item = K>,
    iter_changed_or_insert: impl Iterator<Item = (K, Size)>,
    mut notify_resize: impl FnMut(SizeWithDepth),
    mut notify_change: impl FnMut(K, ValueChange<Option<PackResult2dWithDepth>>),
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
          println!("grow failed, reached max_size: {max:?}");
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
      let previous = if let Some(pack_id) = rev_mapping.remove(&id).unwrap() {
        mapping.remove(&pack_id);
        Some(packer.unpack(pack_id).unwrap())
      } else {
        None
      };
      let delta = ValueChange::Remove(previous);
      notify_change(id, delta);
    }

    for (id, size) in iter_changed_or_insert {
      if let Some(pack_id) = rev_mapping.remove(&id) {
        let previous = if let Some(pack_id) = pack_id {
          mapping.remove(&pack_id);
          Some(packer.unpack(pack_id).unwrap())
        } else {
          None
        };

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
        notify_change(idx, ValueChange::Remove(Some(previous)));

        if let Some(overridden) = mapping.insert(relocation.new.id, idx) {
          staging_mapping.insert(relocation.new.id, overridden);
        }
        let current = relocation.new.result;
        notify_change(idx, ValueChange::Delta(Some(current), None));

        rev_mapping.insert(idx, Some(relocation.new.id));
      };

      let pack_result = packer.pack_and_check_grow(size, &mut grow, &mut relocate);

      if let Ok(pack_result) = pack_result {
        rev_mapping.insert(id, Some(pack_result.id));
        mapping.insert(pack_result.id, id);
        let delta = ValueChange::Delta(Some(pack_result.result), None);

        notify_change(id, delta);
      } else {
        let delta = ValueChange::Delta(None, None);
        notify_change(id, delta);
        rev_mapping.insert(id, None);
        println!("warning, texture allocation failed for {id:?}, try increase the max size")
      }
    }
  }
}
