#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineSegment<T> {
  pub start: T,
  pub end: T,
}

impl<T> LineSegment<T> {
  pub fn new(start: T, end: T) -> Self {
    Self { start, end }
  }

  pub fn iter_point<'a>(&'a self) -> LineSegmentIter<'a, T> {
    LineSegmentIter::new(self)
  }
}

pub struct LineSegmentIter<'a, T> {
  line_segment: &'a LineSegment<T>,
  visit_count: i8,
}

impl<'a, T> LineSegmentIter<'a, T> {
  pub fn new(line3: &'a LineSegment<T>) -> Self {
    Self {
      line_segment: line3,
      visit_count: -1,
    }
  }
}

impl<'a, T: Copy> Iterator for LineSegmentIter<'a, T> {
  type Item = T;
  fn next(&mut self) -> Option<Self::Item> {
    self.visit_count += 1;
    if self.visit_count == 0 {
      Some(self.line_segment.start)
    } else if self.visit_count == 1 {
      Some(self.line_segment.end)
    } else {
      None
    }
  }
}

impl<T: Copy> LineSegment<T> {
  pub fn swap(&self) -> Self {
    Self::new(self.end, self.start)
  }

  pub fn swap_if(&self, prediction: impl FnOnce(Self) -> bool) -> Self {
    if prediction(*self) {
      self.swap()
    } else {
      *self
    }
  }
}
