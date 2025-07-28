#[derive(Default)]
pub struct Adjacency<T> {
  pub offsets: Vec<u32>,
  pub counts: Vec<u32>,
  /// multi data accessed by range
  pub many_data: Vec<T>,
}

impl<T: Default + Clone> Adjacency<T> {
  pub fn from_iter(
    one_side_count: usize,
    one_iter: impl Iterator<Item = u32>,
    many_one_pair_iter: impl Iterator<Item = (T, u32)>,
  ) -> Self {
    let mut offsets = vec![0; one_side_count];
    let mut counts = vec![0; one_side_count];

    let mut many_count = 0;
    for one in one_iter {
      counts[one as usize] += 1;
      many_count += 1;
    }

    // fill offset table
    let mut offset = 0;
    for (o, count) in offsets.iter_mut().zip(counts.iter()) {
      *o = offset;
      offset += *count;
    }

    assert_eq!(offset as usize, many_count);

    let mut many_data = vec![T::default(); many_count];

    for (many, one) in many_one_pair_iter {
      many_data[offsets[one as usize] as usize] = many;
      offsets[one as usize] += 1;
    }

    // fix offsets that have been disturbed by the previous pass
    for (offset, count) in offsets.iter_mut().zip(counts.iter()) {
      assert!(*offset >= *count);
      *offset -= *count;
    }

    Self {
      offsets,
      counts,
      many_data,
    }
  }

  pub fn iter_many_by_one(&self, one: u32) -> impl Iterator<Item = &T> + '_ {
    let one = one as usize;
    let start = self.offsets[one] as usize;
    let count = self.counts[one] as usize;
    self.many_data.get(start..start + count).unwrap().iter()
  }

  /// return if relation is removed
  pub fn try_remove_relation(&mut self, many: &T, one: u32) -> bool
  where
    T: PartialEq,
  {
    let one = one as usize;
    let start = self.offsets[one] as usize;
    let count = self.counts[one] as usize;

    assert!(count > 0);

    let all_many = self.many_data.get_mut(start..start + count).unwrap();
    let last = all_many[count - 1].clone();

    for m in all_many {
      if m == many {
        *m = last; // we do only remove one tri so it's ok not to update last cursor
        self.counts[one] -= 1;
        return true;
      }
    }

    false
  }
}
