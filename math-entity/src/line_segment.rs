use rendiation_math::Vector;

#[derive(Clone, Copy)]
pub struct LineSegment<T, const D: usize> {
  pub start: Vector<T, D>,
  pub end: Vector<T, D>,
}

impl<T, const D: usize> LineSegment<T, D> {
  pub fn new(start: Vector<T, D>, end: Vector<T, D>) -> Self {
    Self { start, end }
  }

  pub fn iter_point<'a>(&'a self) -> LineSegmentIter<'a, T, D> {
    LineSegmentIter::new(self)
  }
}

pub struct LineSegmentIter<'a, T, const D: usize> {
  line_segment: &'a LineSegment<T, D>,
  visit_count: i8,
}

impl<'a, T, const D: usize> LineSegmentIter<'a, T, D> {
  pub fn new(line3: &'a LineSegment<T, D>) -> Self {
    Self {
      line_segment: line3,
      visit_count: -1,
    }
  }
}

impl<'a, T: Copy, const D: usize> Iterator for LineSegmentIter<'a, T, D> {
  type Item = Vector<T, D>;
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

impl<T: Copy, const D: usize> LineSegment<T, D> {
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
