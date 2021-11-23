use std::collections::{HashMap, HashSet};

use rendiation_texture::TextureRange;

use crate::*;

pub struct ShelfPacker {
  config: PackerConfig,

  packed: HashMap<PackId, (TextureRange, usize, usize)>,
  allocator: RowAllocator<Shelf>,
}

impl Default for ShelfPacker {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl ShelfPacker {
  pub fn new(config: PackerConfig) -> Self {
    let (width, height) = config.init_size.into_usize();
    ShelfPacker {
      config,
      packed: Default::default(),
      allocator: RowAllocator::new(Shelf::new(
        Section {
          start: 0,
          extent: height,
        },
        Section {
          start: 0,
          extent: width,
        },
      )),
    }
  }

  fn shelf_creator(&self) -> impl FnOnce(Section) -> Shelf + Copy {
    let width = self.config.init_size.width_usize();
    move |sec: Section| -> Shelf {
      Shelf::new(
        sec,
        Section {
          start: 0,
          extent: width,
        },
      )
    }
  }
}

struct RowAllocator<T> {
  id: usize,
  sections: HashMap<usize, T>,
  free: HashSet<usize>,
}

impl<T: SectionLike> RowAllocator<T> {
  fn new(init: T) -> Self {
    let mut sections = HashMap::new();
    sections.insert(0, init);
    let mut free = HashSet::new();
    free.insert(0);
    Self {
      id: 0,
      sections,
      free,
    }
  }
}

trait SectionLike {
  fn section(&self) -> &Section;
  fn is_empty(&self) -> bool;
}

struct SectionNotExist;

impl<T: SectionLike> RowAllocator<T> {
  pub fn find_or_create_suitable(
    &mut self,
    extent: usize,
    section_creator: impl FnOnce(Section) -> T + Copy,
    section_packable: impl FnOnce(&T) -> bool + Copy,
  ) -> Option<(&mut T, usize)> {
    let mut min: Option<(usize, usize, bool)> = None;
    for section_id in &self.free {
      let section_new = self.sections.get(section_id).unwrap();
      if !section_packable(section_new) {
        continue;
      }

      let is_new_should_split = section_new.is_empty();
      let extend_new = section_new.section().extent;
      if let Some((_, min_extend, should_split)) = min {
        if (!is_new_should_split || should_split) && extend_new >= extent && min_extend > extend_new
        {
          min = (*section_id, extend_new, is_new_should_split).into();
        }
      } else {
        min = (*section_id, extend_new, is_new_should_split).into();
      }
    }

    if let Some((section_id, _, should_split)) = min {
      if should_split {
        self.free.remove(&section_id);
        let section = self.sections.remove(&section_id).unwrap();

        let (top, bottom) = section.section().split(extent);

        let top = section_creator(top);

        if bottom.extent != 0 {
          let bottom = section_creator(bottom);
          self.id += 1;
          self.sections.insert(self.id, bottom);
          self.free.insert(self.id);
        }

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
  pub fn drop_section(
    &mut self,
    section_id: usize,
    section_creator: impl FnOnce(Section) -> T + Copy,
  ) -> Result<bool, SectionNotExist> {
    let section = self.sections.remove(&section_id).ok_or(SectionNotExist)?;
    assert!(section.is_empty()); // todo should we return error?

    let section = *section.section();

    if let Some((new_sec, old_to_remove)) = self.free.iter().find_map(|sec_id| {
      let sec = self.sections.get(&section_id).unwrap();
      section.try_merge(sec.section()).map(|r| (r, *sec_id))
    }) {
      self.sections.remove(&old_to_remove);
      self.free.remove(&old_to_remove);
      self.id += 1;
      self.sections.insert(self.id, section_creator(new_sec));
      self.free.insert(self.id);

      let x_section = self.id;

      if let Some((new_sec, old_to_remove)) = self.free.iter().find_map(|sec_id| {
        let sec = self.sections.get(&section_id).unwrap();
        section.try_merge(sec.section()).map(|r| (r, *sec_id))
      }) {
        self.sections.remove(&x_section);
        self.free.remove(&x_section);

        self.sections.remove(&old_to_remove);
        self.free.remove(&old_to_remove);
        self.id += 1;
        self.sections.insert(self.id, section_creator(new_sec));
        self.free.insert(self.id);
      }
    }

    Ok(self.should_split())
  }

  pub fn should_split(&self) -> bool {
    self.sections.len() == 1 && self.free.len() == 1
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

impl Shelf {
  fn new(v_section: Section, h_section: Section) -> Self {
    Shelf {
      section: v_section,
      allocator: RowAllocator::new(h_section),
    }
  }
}

impl SectionLike for Shelf {
  fn section(&self) -> &Section {
    &self.section
  }
  fn is_empty(&self) -> bool {
    self.allocator.should_split()
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

impl RePackablePacker for ShelfPacker {
  fn pack_with_id(
    &mut self,
    input: rendiation_texture::Size,
  ) -> Result<PackResultWithId, PackError> {
    let width = usize::from(input.width);
    let height = usize::from(input.height);

    let packable = |_shelf: &Shelf| {
      todo!();
    };

    let (row, row_id) = self
      .allocator
      .find_or_create_suitable(height, self.shelf_creator(), packable)
      .ok_or(PackError::SpaceNotEnough)?;

    let (section, section_id) = row
      .allocator
      .find_or_create_suitable(width, Section::from, |_| true)
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

    let shelf_should_split = shelf
      .allocator
      .drop_section(section_id, Section::from)
      .map_err(|_| UnpackError::UnpackItemNotExist)?;

    if shelf_should_split {
      self
        .allocator
        .drop_section(shelf_id, self.shelf_creator())
        .map_err(|_| UnpackError::UnpackItemNotExist)?;
    }

    Ok(())
  }
}
