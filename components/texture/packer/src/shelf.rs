use std::collections::HashMap;

use rendiation_texture::TextureRange;

use crate::*;

#[derive(Default)]
pub struct ShelfPacker {
  config: PackerConfig,

  packed: HashMap<PackId, (TextureRange, usize, usize)>,
  allocator: RowAllocator<Shelf>,
}

impl ShelfPacker {
  pub fn new(config: PackerConfig) -> Self {
    ShelfPacker {
      config,
      packed: Default::default(),
      allocator: Default::default(),
    }
  }
}

// todo optimize use link list and heap
struct RowAllocator<T> {
  id: usize,
  sections: HashMap<usize, T>,
}

impl<T> Default for RowAllocator<T> {
  fn default() -> Self {
    Self {
      id: 0,
      sections: Default::default(),
    }
  }
}

trait SectionLike: From<Section> {
  fn section(&self) -> &Section;
  fn is_empty(&self) -> bool;
}

struct SectionNotExist;

impl<T: SectionLike> RowAllocator<T> {
  pub fn find_or_create_suitable(&mut self, extent: usize) -> Option<(&mut T, usize)> {
    let mut min: Option<(usize, usize, bool)> = None;
    for (section_id, section_new) in &mut self.sections {
      let is_empty_new = section_new.is_empty();
      let extend_new = section_new.section().extent;
      if let Some((_, min_extend, is_empty)) = min {
        if (!is_empty_new || is_empty) && extend_new >= extent && min_extend > extend_new {
          min = (*section_id, extend_new, is_empty_new).into();
        }
      } else {
        min = (*section_id, extend_new, is_empty_new).into();
      }
    }

    if let Some((section_id, _, is_empty)) = min {
      if is_empty {
        let section = self.sections.remove(&section_id).unwrap();

        let (top, bottom) = section.section().split(extent);

        let top = top.into();
        let bottom = bottom.into();

        self.id += 1;
        self.sections.insert(self.id, bottom);

        self.id += 1;
        let section_id = self.id;
        let section = self.sections.entry(section_id).or_insert(top);

        (section, section_id).into()
      } else {
        let section = self.sections.get_mut(&section_id).unwrap();
        (section, section_id).into()
      }
    } else {
      None
    }
  }

  pub fn get_section_mut(&mut self, section: usize) -> Result<&mut T, SectionNotExist> {
    self.sections.get_mut(&section).ok_or(SectionNotExist)
  }

  /// The adjacent empty section will be merged
  ///
  /// return if is empty after drop
  pub fn drop_section(&mut self, section_id: usize) -> Result<bool, SectionNotExist> {
    let section = self.sections.get(&section_id).ok_or(SectionNotExist)?;
    assert!(section.is_empty()); // todo should we return error?
    let section = *section.section();

    if let Some((new_sec, old_to_remove)) = self
      .sections
      .iter()
      .find_map(|(sec_id, sec)| section.try_merge(sec.section()).map(|r| (r, *sec_id)))
    {
      self.sections.remove(&old_to_remove);
      self.id += 1;
      self.sections.insert(self.id, new_sec.into());
    }

    if let Some((new_sec, old_to_remove)) = self
      .sections
      .iter()
      .find_map(|(sec_id, sec)| section.try_merge(sec.section()).map(|r| (r, *sec_id)))
    {
      self.sections.remove(&old_to_remove);
      self.id += 1;
      self.sections.insert(self.id, new_sec.into());
    }

    Ok(self.sections.len() == 1 && self.sections.iter().next().unwrap().1.is_empty())
  }

  pub fn is_empty(&self) -> bool {
    self.sections.is_empty()
  }
}

#[derive(Clone, Copy)]
pub struct Section {
  start: usize,
  extent: usize,
}

impl Section {
  pub fn split(&self, extent: usize) -> (Section, Section) {
    assert!(extent < self.extent);
    (
      Section {
        start: self.start,
        extent,
      },
      Section {
        start: self.start + extent,
        extent: self.extent - extent,
      },
    )
  }

  pub fn try_merge(&self, other: &Self) -> Option<Self> {
    let self_end = self.start + self.extent;
    let other_end = other.start + other.extent;
    if self.start == other_end {
      Self {
        start: other.start,
        extent: self.extent + other.extent,
      }
      .into()
    } else if self_end == other.start {
      Self {
        start: self.start,
        extent: self.extent + other.extent,
      }
      .into()
    } else {
      None
    }
  }
}

impl SectionLike for Section {
  fn section(&self) -> &Section {
    self
  }
  fn is_empty(&self) -> bool {
    true
  }
}

pub struct Shelf {
  section: Section,
  allocator: RowAllocator<Section>,
}

impl From<Section> for Shelf {
  fn from(section: Section) -> Self {
    Shelf {
      section,
      allocator: Default::default(),
    }
  }
}

impl SectionLike for Shelf {
  fn section(&self) -> &Section {
    &self.section
  }
  fn is_empty(&self) -> bool {
    self.allocator.is_empty()
  }
}

impl BaseTexturePacker for ShelfPacker {
  fn config(&mut self, config: PackerConfig) {
    self.config = config;
    self.reset();
  }

  fn reset(&mut self) {
    *self = Self::new(self.config)
  }
}

/// https://github.com/alexheretic/glyph-brush/blob/master/draw-cache/src/lib.rs
impl RePackablePacker for ShelfPacker {
  fn pack_with_id(
    &mut self,
    input: rendiation_texture::Size,
  ) -> Result<PackResultWithId, PackError> {
    let width = usize::from(input.width);
    let height = usize::from(input.height);

    let (row, row_id) = self
      .allocator
      .find_or_create_suitable(height)
      .ok_or(PackError::SpaceNotEnough)?;

    let (section, section_id) = row
      .allocator
      .find_or_create_suitable(width)
      .ok_or(PackError::SpaceNotEnough)?;

    let range = TextureRange {
      origin: (section.start, row.section.start).into(),
      size: input,
    };

    let id = Default::default();
    self.packed.insert(id, (range, row_id, section_id));

    Ok(PackResultWithId {
      result: PackResult {
        range,
        rotated: false,
      },
      id,
    })
  }

  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    let (_result, shelf_id, section_id) = self
      .packed
      .remove(&id)
      .ok_or(UnpackError::UnpackItemNotExist)?;

    let shelf = self
      .allocator
      .get_section_mut(shelf_id)
      .map_err(|_| UnpackError::UnpackItemNotExist)?;

    let shelf_is_empty = shelf
      .allocator
      .drop_section(section_id)
      .map_err(|_| UnpackError::UnpackItemNotExist)?;

    if shelf_is_empty {
      self
        .allocator
        .drop_section(shelf_id)
        .map_err(|_| UnpackError::UnpackItemNotExist)?;
    }

    Ok(())
  }
}
