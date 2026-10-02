use std::ops::Range;

/// the ranges with a smaller gap are merged, re-uploading a small clean part is cheaper than an
/// extra texture write call
const MERGE_GAP_TEXELS: usize = 1024;
const INIT_MERGE_THRESHOLD: usize = 1024;

/// The dirty texel ranges that wait for upload.
///
/// The ranges are merged when the count reaches the threshold, so the memory is bounded even if
/// the buffer is not flushed for a long time.
pub(crate) struct DirtyTexelRanges {
  ranges: Vec<Range<usize>>,
  merge_threshold: usize,
}

impl Default for DirtyTexelRanges {
  fn default() -> Self {
    Self {
      ranges: Vec::new(),
      merge_threshold: INIT_MERGE_THRESHOLD,
    }
  }
}

impl DirtyTexelRanges {
  pub fn push(&mut self, range: Range<usize>) {
    if range.is_empty() {
      return;
    }
    self.ranges.push(range);
    if self.ranges.len() >= self.merge_threshold {
      self.merge();
      // avoid merging on every push if the ranges are too sparse to merge
      self.merge_threshold = (self.ranges.len() * 2).max(INIT_MERGE_THRESHOLD);
    }
  }

  pub fn is_empty(&self) -> bool {
    self.ranges.is_empty()
  }

  /// return the sorted and merged ranges, and clear self
  pub fn take(&mut self) -> Vec<Range<usize>> {
    self.merge();
    self.merge_threshold = INIT_MERGE_THRESHOLD;
    std::mem::take(&mut self.ranges)
  }

  fn merge(&mut self) {
    self.ranges.sort_unstable_by_key(|r| r.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(self.ranges.len());
    for range in self.ranges.drain(..) {
      if let Some(last) = merged.last_mut()
        && range.start <= last.end + MERGE_GAP_TEXELS
      {
        last.end = last.end.max(range.end);
      } else {
        merged.push(range);
      }
    }
    self.ranges = merged;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn merge_overlapped_and_close_ranges() {
    let mut dirty = DirtyTexelRanges::default();
    let far = 10 * MERGE_GAP_TEXELS;
    dirty.push(far..far + 4);
    dirty.push(5..10);
    dirty.push(0..6);
    dirty.push(20..20);
    dirty.push(10 + MERGE_GAP_TEXELS..12 + MERGE_GAP_TEXELS);
    dirty.push(8..9);
    assert_eq!(dirty.take(), vec![0..12 + MERGE_GAP_TEXELS, far..far + 4]);
    assert!(dirty.is_empty());
  }

  #[test]
  fn sparse_ranges_are_bounded() {
    let mut dirty = DirtyTexelRanges::default();
    let stride = 2 * MERGE_GAP_TEXELS;
    for round in 0..4 {
      for i in 0..INIT_MERGE_THRESHOLD * 4 {
        dirty.push(i * stride + round..i * stride + round + 1);
      }
    }
    assert!(dirty.ranges.len() <= INIT_MERGE_THRESHOLD * 8);
    let ranges = dirty.take();
    assert_eq!(ranges.len(), INIT_MERGE_THRESHOLD * 4);
    assert_eq!(ranges[1], stride..stride + 4);
  }
}
