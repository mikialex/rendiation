use parametric_normalized_model::Axis;

/// Parameters are dyadic numbers `n / 2^PARAM_BITS` stored as integers, so every split point,
/// leaf grid point and their comparisons are exact (also in f32 on the GPU).
pub const PARAM_BITS: u32 = 20;
pub const PARAM_ONE: u32 = 1 << PARAM_BITS;

pub fn param_to_f64(n: u32) -> f64 {
  n as f64 / PARAM_ONE as f64
}

/// The coarsest subdivision level at which `n` is a split point (0 for the domain ends).
pub fn param_level(n: u32) -> u32 {
  if n == 0 || n == PARAM_ONE {
    0
  } else {
    PARAM_BITS - n.trailing_zeros()
  }
}

/// The dyadic interval `[index, index + 1] / 2^level` of the parameter domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Segment {
  pub level: u32,
  pub index: u32,
}

impl Segment {
  pub const ROOT: Self = Self { level: 0, index: 0 };

  pub fn start(self) -> u32 {
    self.index << (PARAM_BITS - self.level)
  }

  pub fn end(self) -> u32 {
    (self.index + 1) << (PARAM_BITS - self.level)
  }

  pub fn length(self) -> u32 {
    1 << (PARAM_BITS - self.level)
  }

  /// The segment of `level` containing `p` in its interior, `None` if `p` is a split point of
  /// that level.
  pub fn containing(level: u32, p: u32) -> Option<Self> {
    let shift = PARAM_BITS - level;
    (p & ((1 << shift) - 1) != 0).then_some(Self {
      level,
      index: p >> shift,
    })
  }

  /// The ancestor (or self) at `level <= self.level`.
  pub fn ancestor(self, level: u32) -> Self {
    Self {
      level,
      index: self.index >> (self.level - level),
    }
  }

  /// The same interval with the parameter direction reversed (`p -> 1 - p`).
  pub fn reversed(self) -> Self {
    Self {
      level: self.level,
      index: (1 << self.level) - 1 - self.index,
    }
  }

  pub fn children(self) -> [Self; 2] {
    let level = self.level + 1;
    [
      Self {
        level,
        index: self.index * 2,
      },
      Self {
        level,
        index: self.index * 2 + 1,
      },
    ]
  }
}

/// A work item or leaf: the sub patch `[ix, ix + 1] × [iy, iy + 1] / 2^level` of item `item`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubPatchKey {
  pub item: u32,
  pub level: u32,
  pub ix: u32,
  pub iy: u32,
}

impl SubPatchKey {
  pub fn root(item: u32) -> Self {
    Self {
      item,
      level: 0,
      ix: 0,
      iy: 0,
    }
  }

  pub fn u_segment(self) -> Segment {
    Segment {
      level: self.level,
      index: self.ix,
    }
  }

  pub fn v_segment(self) -> Segment {
    Segment {
      level: self.level,
      index: self.iy,
    }
  }

  pub fn children(self) -> [Self; 4] {
    let (level, ix, iy) = (self.level + 1, self.ix * 2, self.iy * 2);
    [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| Self {
      item: self.item,
      level,
      ix: ix + dx,
      iy: iy + dy,
    })
  }

  /// The four boundary edges as (iso line, segment along the line in patch parameter
  /// direction), in the order u start, u end, v start, v end.
  pub fn edges(self) -> [(LineKey, Segment); 4] {
    let (us, vs) = (self.u_segment(), self.v_segment());
    [
      (LineKey::new(Axis::U, us.start()), vs),
      (LineKey::new(Axis::U, us.end()), vs),
      (LineKey::new(Axis::V, vs.start()), us),
      (LineKey::new(Axis::V, vs.end()), us),
    ]
  }
}

/// An iso line of an original patch: `u = coord` for `Axis::U` (parameterized by v), `v = coord`
/// for `Axis::V` (parameterized by u).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineKey {
  pub axis: Axis,
  pub coord: u32,
}

impl LineKey {
  pub fn new(axis: Axis, coord: u32) -> Self {
    Self { axis, coord }
  }

  pub fn is_patch_boundary(self) -> bool {
    self.coord == 0 || self.coord == PARAM_ONE
  }

  /// The subdivision level where this line first becomes a sub patch edge. Only segments
  /// inside one cell of this level are considered for its linearization, see `crate::line`.
  pub fn level(self) -> u32 {
    param_level(self.coord)
  }

  /// Patch domain point at parameter `along` of this line.
  pub fn point(self, along: u32) -> (u32, u32) {
    match self.axis {
      Axis::U => (self.coord, along),
      Axis::V => (along, self.coord),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn segments() {
    assert_eq!(param_level(0), 0);
    assert_eq!(param_level(PARAM_ONE), 0);
    assert_eq!(param_level(PARAM_ONE / 2), 1);
    assert_eq!(param_level(PARAM_ONE / 4 * 3), 2);
    let s = Segment { level: 2, index: 1 };
    assert_eq!((s.start(), s.end()), (PARAM_ONE / 4, PARAM_ONE / 2));
    assert_eq!(s.reversed(), Segment { level: 2, index: 2 });
    assert_eq!(s.ancestor(1), Segment { level: 1, index: 0 });
    assert_eq!(Segment::containing(2, PARAM_ONE / 4), None);
    assert_eq!(Segment::containing(2, PARAM_ONE / 4 + 1), Some(s));
    assert_eq!(Segment::containing(0, 0), None);
  }
}
