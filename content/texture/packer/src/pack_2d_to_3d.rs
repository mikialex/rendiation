use std::iter;

use fast_hash_collection::FastHashMap;
use rendiation_texture_core::{Size, SizeWithDepth};

use crate::pack_2d_to_2d::*;
use crate::*;

pub struct MultiLayerTexturePacker<P> {
  packers: Vec<P>,
  next_id: u32,
  id_remap: FastHashMap<PackId, (usize, PackId)>,
}

impl<P> TexturePackerInit for MultiLayerTexturePacker<P>
where
  P: RePackablePacker + TexturePackerInit<Config = PackerConfig2d> + Default,
{
  type Config = SizeWithDepth;

  fn init_by_config(config: Self::Config) -> Self {
    let single_config = PackerConfig2d {
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
}

impl<P> RePackablePacker for MultiLayerTexturePacker<P>
where
  P: RePackablePacker<Input = Size, PackOutput = PackResult2d>,
{
  type Input = Size;
  type PackOutput = PackResult2dWithDepth;
  fn pack_with_id(
    &mut self,
    input: Size,
  ) -> Result<PackResultWithId<PackResult2dWithDepth>, PackError> {
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
      self.id_remap.insert(id, (idx, re.id));
      let result = PackResultWithId {
        result: PackResult2dWithDepth {
          result: re.result,
          depth: idx as u32,
        },
        id,
      };
      Ok(result)
    } else {
      Err(PackError::SpaceNotEnough)
    }
  }
  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    let (idx, pack) = self
      .id_remap
      .remove(&id)
      .ok_or(UnpackError::UnpackItemNotExist)?;
    self.packers[idx].unpack(pack)
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackResult2dWithDepth {
  pub result: PackResult2d,
  pub depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiLayerTexturePackerConfig {
  pub max_size: SizeWithDepth,
  pub init_size: SizeWithDepth,
}
