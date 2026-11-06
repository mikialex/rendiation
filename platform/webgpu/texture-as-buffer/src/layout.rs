/// The texel size of the texture that stores a u32 heap.
///
/// The heap is stored in row major order. The extent is either a single row, or rows of the max
/// width, so when the extent grows the existing texels keep their position and the old texture
/// can be copied into the new one by a single rect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexelExtent {
  pub width: u32,
  pub height: u32,
}

impl TexelExtent {
  /// The smallest extent that holds `texel_count` texels, return None if it exceeds the limit.
  ///
  /// The single row width is rounded up to the power of two to amortize the reallocation when the
  /// data grows within one row.
  pub fn required(texel_count: u64, limit: TexelExtent) -> Option<Self> {
    let texel_count = texel_count.max(1);
    let max_width = limit.width as u64;
    if texel_count <= max_width {
      let width = texel_count.next_power_of_two().min(max_width) as u32;
      Some(Self { width, height: 1 })
    } else {
      let height = texel_count.div_ceil(max_width);
      (height <= limit.height as u64).then_some(Self {
        width: limit.width,
        height: height as u32,
      })
    }
  }

  pub fn texel_count(&self) -> u64 {
    self.width as u64 * self.height as u64
  }

  pub fn contains(&self, other: &Self) -> bool {
    self.width >= other.width && self.height >= other.height
  }
}

/// A rect copy between two row major texel layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexelCopyRect {
  pub src: (u32, u32),
  pub dst: (u32, u32),
  pub width: u32,
  pub height: u32,
  /// the offset of the first texel of this rect from the start of the linear range
  pub linear_offset: u64,
}

/// Split the copy of a linear texel range into rects, the src and dst may have different row width.
///
/// When both sides have the same row width and the same in-row offset, at most 3 rects are
/// produced: the head, the full rows in the middle, and the tail. Otherwise each row may be split
/// into 2 rects, so the rect count is proportional to the row count.
pub fn split_linear_copy(
  src_start: u64,
  src_width: u32,
  dst_start: u64,
  dst_width: u32,
  count: u64,
  mut f: impl FnMut(TexelCopyRect),
) {
  let (sw, dw) = (src_width as u64, dst_width as u64);
  let mut progress = 0;
  while progress < count {
    let (s, d) = (src_start + progress, dst_start + progress);
    let (sx, sy, dx, dy) = (s % sw, s / sw, d % dw, d / dw);
    let remain = count - progress;

    let (width, height) = if sx == 0 && dx == 0 && sw == dw && remain >= sw {
      (sw, remain / sw)
    } else {
      (remain.min(sw - sx).min(dw - dx), 1)
    };

    f(TexelCopyRect {
      src: (sx as u32, sy as u32),
      dst: (dx as u32, dy as u32),
      width: width as u32,
      height: height as u32,
      linear_offset: progress,
    });
    progress += width * height;
  }
}

/// Split the linear texel range into at most 3 row major rects, the full rows in the middle are
/// merged into one rect, so the data of each rect is contiguous in the linear source.
pub fn split_linear_range(start: u64, width: u32, count: u64, f: impl FnMut(TexelCopyRect)) {
  split_linear_copy(start, width, start, width, count, f)
}

#[cfg(test)]
mod tests {
  use super::*;

  const LIMIT: TexelExtent = TexelExtent {
    width: 8,
    height: 4,
  };

  #[test]
  fn required_extent() {
    let expect = [
      (0, Some((1, 1))),
      (1, Some((1, 1))),
      (3, Some((4, 1))),
      (8, Some((8, 1))),
      (9, Some((8, 2))),
      (16, Some((8, 2))),
      (17, Some((8, 3))),
      (32, Some((8, 4))),
      (33, None),
    ];
    for (count, extent) in expect {
      let extent = extent.map(|(width, height)| TexelExtent { width, height });
      assert_eq!(TexelExtent::required(count, LIMIT), extent, "count {count}");
    }

    let limit = TexelExtent {
      width: 6,
      height: 4,
    };
    let r = TexelExtent::required(5, limit).unwrap();
    assert_eq!((r.width, r.height), (6, 1));
  }

  #[test]
  fn required_extent_keeps_layout_when_grow() {
    // the smaller extent must be either the same width, or a single row
    for a in 0..=32 {
      for b in a..=32 {
        let ea = TexelExtent::required(a, LIMIT).unwrap();
        let eb = TexelExtent::required(b, LIMIT).unwrap();
        assert!(eb.contains(&ea));
        assert!(ea.width == eb.width || ea.height == 1);
        assert!(eb.texel_count() >= b);
      }
    }
  }

  fn simulate_copy(src_width: u32, dst_width: u32, src_start: u64, dst_start: u64, count: u64) {
    let len = 64;
    let src: Vec<_> = (0..len).map(|v| v + 1000).collect();
    let mut dst = vec![0; len as usize];
    let mut rect_count = 0;

    split_linear_copy(src_start, src_width, dst_start, dst_width, count, |r| {
      rect_count += 1;
      assert!(r.src.0 + r.width <= src_width);
      assert!(r.dst.0 + r.width <= dst_width);
      assert!(r.height == 1 || (r.src.0 == 0 && r.dst.0 == 0 && r.width == src_width));
      let src_linear = r.src.1 as u64 * src_width as u64 + r.src.0 as u64;
      assert_eq!(src_linear, src_start + r.linear_offset);
      for y in 0..r.height {
        for x in 0..r.width {
          let s = (r.src.1 + y) as usize * src_width as usize + (r.src.0 + x) as usize;
          let d = (r.dst.1 + y) as usize * dst_width as usize + (r.dst.0 + x) as usize;
          dst[d] = src[s];
        }
      }
    });

    let mut expect = vec![0; len as usize];
    for i in 0..count {
      expect[(dst_start + i) as usize] = src[(src_start + i) as usize];
    }
    assert_eq!(dst, expect);

    let same_alignment =
      src_width == dst_width && src_start % src_width as u64 == dst_start % dst_width as u64;
    if same_alignment {
      assert!(rect_count <= 3, "rect count {rect_count}");
    }
  }

  #[test]
  fn linear_copy_split() {
    for (src_width, dst_width) in [(8, 8), (4, 8), (8, 4), (3, 8)] {
      for src_start in 0..20 {
        for dst_start in 0..20 {
          for count in 0..=(64 - src_start.max(dst_start)).min(40) {
            simulate_copy(src_width, dst_width, src_start, dst_start, count);
          }
        }
      }
    }
  }

  #[test]
  fn linear_range_split() {
    let mut rects = Vec::new();
    split_linear_range(5, 8, 20, |r| rects.push(r));
    let expect: Vec<_> = [((5, 0), 3, 1, 0), ((0, 1), 8, 2, 3), ((0, 3), 1, 1, 19)]
      .into_iter()
      .map(|(pos, width, height, linear_offset)| TexelCopyRect {
        src: pos,
        dst: pos,
        width,
        height,
        linear_offset,
      })
      .collect();
    assert_eq!(rects, expect);
  }
}
