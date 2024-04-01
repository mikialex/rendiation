use std::{iter, num::NonZeroU32};

use fast_hash_collection::FastHashMap;
use rendiation_texture::Size;

use crate::pack_2d_to_2d::*;

pub struct MultiLayerTexturePacker<P> {
  packers: Vec<P>,
  next_id: u32,
  id_remap: FastHashMap<PackId, (usize, PackResultWithId)>,
}

impl<P: RePackablePacker + TexturePackerInit> MultiLayerTexturePacker<P> {
  pub fn new(config: SizeWithDepth) -> Self {
    let single_config = PackerConfig {
      allow_90_rotation: false,
      full_size: config.size,
    };
    let depth: u32 = config.depth.into();
    let packers = iter::repeat_with(|| P::init_by_config(single_config))
      .take(depth as usize)
      .collect();

    Self {
      packers,
      next_id: 0,
      id_remap: Default::default(),
    }
  }

  pub fn pack_with_id(&mut self, input: Size) -> Result<PackResultWithIdWithDepth, PackError> {
    let mut result = None;
    // todo, maybe reorder packer to reduce cost
    for (idx, packer) in self.packers.iter_mut().enumerate() {
      if let Ok(sub_result) = packer.pack_with_id(input) {
        result = Some((idx, sub_result));
      }
    }
    if let Some((idx, re)) = result {
      self.next_id += 1;
      let id = PackId(self.next_id);
      self.id_remap.insert(id, (idx, re));
      Ok(PackResultWithIdWithDepth {
        result: PackResultWithId {
          result: re.result,
          id,
        },
        depth: idx as u32,
      })
    } else {
      Err(PackError::SpaceNotEnough)
    }
  }
  pub fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    let (idx, pack) = self
      .id_remap
      .remove(&id)
      .ok_or(UnpackError::UnpackItemNotExist)?;
    self.packers[idx].unpack(pack.id)
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackResultWithIdWithDepth {
  pub result: PackResultWithId,
  pub depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiLayerTexturePackerConfig {
  pub max_size: SizeWithDepth,
  pub init_size: SizeWithDepth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeWithDepth {
  pub depth: NonZeroU32,
  pub size: Size,
}
