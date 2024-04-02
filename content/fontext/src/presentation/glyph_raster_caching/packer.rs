use rendiation_texture_packer::pack_2d_to_2d::{PackResult2d, PackerConfig2d};
use rendiation_texture_packer::{PackError, TexturePackerInit, UnpackError};
use rendiation_texture_packer::{PackId, PackResultWithId, RePackablePacker};

use crate::*;

pub trait GlyphPackerImpl: 'static {
  fn config(&mut self, config: PackerConfig2d);
  fn pack_with_id(&mut self, input: Size) -> Result<PackResultWithId<PackResult2d>, PackError>;
  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError>;
}

impl<T> GlyphPackerImpl for T
where
  T: RePackablePacker<Input = Size, PackOutput = PackResult2d>
    + TexturePackerInit<Config = PackerConfig2d>
    + 'static,
{
  fn config(&mut self, config: PackerConfig2d) {
    *self = Self::init_by_config(config);
  }

  fn pack_with_id(&mut self, id: Size) -> Result<PackResultWithId<PackResult2d>, PackError> {
    (*self).pack_with_id(id)
  }

  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    (*self).unpack(id)
  }
}

pub struct GlyphPacker {
  packer: Box<dyn GlyphPackerImpl>,
  pack_info: LinkedHashMap<(FontGlyphId, NormalizedGlyphRasterInfo), (PackId, TextureRange)>,
}

impl GlyphPacker {
  pub fn init(init_size: Size, mut packer: impl GlyphPackerImpl + 'static) -> Self {
    packer.config(PackerConfig2d {
      allow_90_rotation: false,
      full_size: init_size,
    });
    Self {
      packer: Box::new(packer),
      pack_info: Default::default(),
    }
  }

  pub fn re_init(&mut self, init_size: Size) {
    self.packer.config(PackerConfig2d {
      allow_90_rotation: false,
      full_size: init_size,
    });
    self.pack_info = Default::default();
  }

  pub fn get_packed(&self, key: &(FontGlyphId, NormalizedGlyphRasterInfo)) -> Option<TextureRange> {
    self.pack_info.get(key).map(|(_, range)| *range)
  }

  pub fn process_queued<'a>(
    &'a mut self,
    queue: &'a FastHashMap<(FontGlyphId, NormalizedGlyphRasterInfo), GlyphRasterInfo>,
  ) -> GlyphPackFrameTask<'a> {
    GlyphPackFrameTask {
      packer: self,
      queue,
    }
  }
}

pub struct GlyphPackFrameTask<'a> {
  packer: &'a mut GlyphPacker,
  queue: &'a FastHashMap<(FontGlyphId, NormalizedGlyphRasterInfo), GlyphRasterInfo>,
}

impl<'a> GlyphPackFrameTask<'a> {
  pub fn rebuild_all(&mut self, new_size: Size) {
    self.packer.re_init(new_size);
  }

  pub fn pack(
    &mut self,
    id: FontGlyphId,
    info: NormalizedGlyphRasterInfo,
    raw_info: GlyphRasterInfo,
    fonts: &FontManager,
  ) -> GlyphAddCacheResult {
    if let Some(result) = self.packer.pack_info.get_refresh(&(id, info)) {
      GlyphAddCacheResult::AlreadyCached(*result)
    } else if let Some(data) = fonts.raster(id, raw_info) {
      loop {
        match self.packer.packer.pack_with_id(data.size()) {
          Ok(result) => {
            let range = result.result.range;

            let result = *self
              .packer
              .pack_info
              .entry((id, info))
              .or_insert((result.id, range));

            break GlyphAddCacheResult::NewCached { result, data };
          }
          Err(err) => match err {
            PackError::SpaceNotEnough => {
              if let Some((k, _)) = self.packer.pack_info.back() {
                if self.queue.contains_key(k) {
                  break GlyphAddCacheResult::NotEnoughSpace;
                } else {
                  let (_, v) = self.packer.pack_info.pop_front().unwrap();
                  self.packer.packer.unpack(v.0).expect("glyph unpack error");
                }
              } else {
                break GlyphAddCacheResult::NotEnoughSpace;
              }
            }
          },
        }
      }
    } else {
      GlyphAddCacheResult::NoGlyphRasterized
    }
  }
}

pub enum GlyphAddCacheResult {
  NewCached {
    result: (PackId, TextureRange),
    data: Texture2DBuffer<u8>,
  },
  NoGlyphRasterized,
  AlreadyCached((PackId, TextureRange)),
  NotEnoughSpace,
}
