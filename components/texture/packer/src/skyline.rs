// https://github.com/PistonDevelopers/texture_packer/blob/master/src/packer/skyline_packer.rs

use rendiation_texture::Size;

use crate::*;

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug)]
pub struct Rect {
  /// Horizontal position the rectangle begins at.
  pub x: usize,
  /// Vertical position the rectangle begins at.
  pub y: usize,
  /// Width of the rectangle.
  pub w: usize,
  /// Height of the rectangle.
  pub h: usize,
}

impl Rect {
  /// Create a new [Rect] based on a position and its width and height.
  pub fn new(x: usize, y: usize, w: usize, h: usize) -> Rect {
    Rect { x, y, w, h }
  }

  /// Get the top coordinate of the rectangle.
  #[inline(always)]
  pub fn top(&self) -> usize {
    self.y
  }

  /// Get the bottom coordinate of the rectangle.
  #[inline(always)]
  pub fn bottom(&self) -> usize {
    self.y + self.h - 1
  }

  /// Get the left coordinate of the rectangle.
  #[inline(always)]
  pub fn left(&self) -> usize {
    self.x
  }

  /// Get the right coordinate of the rectangle.
  #[inline(always)]
  pub fn right(&self) -> usize {
    self.x + self.w - 1
  }

  /// Check if this rectangle contains another.
  pub fn contains(&self, other: &Rect) -> bool {
    self.left() <= other.left()
      && self.right() >= other.right()
      && self.top() <= other.top()
      && self.bottom() >= other.bottom()
  }
}

struct Skyline {
  pub x: usize,
  pub y: usize,
  pub w: usize,
}

impl Skyline {
  #[inline(always)]
  pub fn left(&self) -> usize {
    self.x
  }

  #[inline(always)]
  pub fn right(&self) -> usize {
    self.x + self.w - 1
  }
}

pub struct SkylinePacker {
  config: PackerConfig,
  border: Rect,

  // the skylines are sorted by their `x` position
  skylines: Vec<Skyline>,
}

impl SkylinePacker {
  pub fn new(config: PackerConfig) -> Self {
    let skylines = vec![Skyline {
      x: 0,
      y: 0,
      w: config.init_size.width.into(),
    }];

    SkylinePacker {
      config,
      border: Rect::new(
        0,
        0,
        config.init_size.width.into(),
        config.init_size.height.into(),
      ),
      skylines,
    }
  }

  // return `rect` if rectangle (w, h) can fit the skyline started at `i`
  fn can_put(&self, mut i: usize, w: usize, h: usize) -> Option<Rect> {
    let mut rect = Rect::new(self.skylines[i].x, 0, w, h);
    let mut width_left = rect.w;
    loop {
      rect.y = std::cmp::max(rect.y, self.skylines[i].y);
      // the source rect is too large
      if !self.border.contains(&rect) {
        return None;
      }
      if self.skylines[i].w >= width_left {
        return Some(rect);
      }
      width_left -= self.skylines[i].w;
      i += 1;
      assert!(i < self.skylines.len());
    }
  }

  fn find_skyline(&self, size: Size) -> Option<(usize, Rect)> {
    let w: usize = size.width.into();
    let h: usize = size.height.into();

    let mut bottom = std::usize::MAX;
    let mut width = std::usize::MAX;
    let mut index = None;
    let mut rect = Rect::new(0, 0, 0, 0);

    // keep the `bottom` and `width` as small as possible
    for i in 0..self.skylines.len() {
      if let Some(r) = self.can_put(i, w, h) {
        if r.bottom() < bottom || (r.bottom() == bottom && self.skylines[i].w < width) {
          bottom = r.bottom();
          width = self.skylines[i].w;
          index = Some(i);
          rect = r;
        }
      }

      if self.config.allow_90_rotation {
        if let Some(r) = self.can_put(i, h, w) {
          if r.bottom() < bottom || (r.bottom() == bottom && self.skylines[i].w < width) {
            bottom = r.bottom();
            width = self.skylines[i].w;
            index = Some(i);
            rect = r;
          }
        }
      }
    }

    index.map(|x| (x, rect))
  }

  fn split(&mut self, index: usize, rect: &Rect) {
    let skyline = Skyline {
      x: rect.left(),
      y: rect.bottom() + 1,
      w: rect.w,
    };

    assert!(skyline.right() <= self.border.right());
    assert!(skyline.y <= self.border.bottom());

    self.skylines.insert(index, skyline);

    let i = index + 1;
    while i < self.skylines.len() {
      assert!(self.skylines[i - 1].left() <= self.skylines[i].left());

      if self.skylines[i].left() <= self.skylines[i - 1].right() {
        let shrink = self.skylines[i - 1].right() - self.skylines[i].left() + 1;
        if self.skylines[i].w <= shrink {
          self.skylines.remove(i);
        } else {
          self.skylines[i].x += shrink;
          self.skylines[i].w -= shrink;
          break;
        }
      } else {
        break;
      }
    }
  }

  fn merge(&mut self) {
    let mut i = 1;
    while i < self.skylines.len() {
      if self.skylines[i - 1].y == self.skylines[i].y {
        self.skylines[i - 1].w += self.skylines[i].w;
        self.skylines.remove(i);
        i -= 1;
      }
      i += 1;
    }
  }
}

impl TexturePacker for SkylinePacker {
  fn pack(&mut self, input: Size) -> Result<PackResult, PackError> {
    if let Some((i, rect)) = self.find_skyline(input) {
      self.split(i, &rect);
      self.merge();

      let width: usize = input.width.into();
      let rotated = width != rect.w;

      Ok(PackResult {
        range: TextureRange {
          origin: (rect.x, rect.y).into(),
          size: Size::from_usize_pair_min_one((rect.w, rect.h)),
        },
        rotated,
      })
    } else {
      Err(PackError::SpaceNotEnough)
    }
  }
}

impl BaseTexturePacker for SkylinePacker {
  fn config(&mut self, config: PackerConfig) {
    self.config = config;
    self.reset();
  }

  fn reset(&mut self) {
    *self = Self::new(self.config)
  }
}

impl PackableChecker for SkylinePacker {
  fn can_pack(&self, input: Size) -> bool {
    if let Some((_, rect)) = self.find_skyline(input) {
      let skyline = Skyline {
        x: rect.left(),
        y: rect.bottom() + 1,
        w: rect.w,
      };

      return skyline.right() <= self.border.right() && skyline.y <= self.border.bottom();
    }
    false
  }
}
