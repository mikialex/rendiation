use rendiation_algebra::{Vec3, Vector};

// todo use upstream
pub trait CenterAblePrimitive {
  type Center;
  fn get_center(&self) -> Self::Center;
}

pub trait QbvhDataSplitter<BV> {
  fn split_dataset<'idx>(
    &mut self,
    indices: &'idx mut [usize],
    proxies: &[BV],
  ) -> [&'idx mut [usize]; 4];
}

type CenterOf<BV> = <BV as CenterAblePrimitive>::Center;
/// A data splitter that arranges a set of Aabbs in two sets based on their center’s coordinate
/// along the split axis.
pub struct CenterDataSplitter<const D: usize> {
  /// If all the Aabb centers have the same coordinate values along the splitting axis
  /// setting this to `true` will allow the splitter to split the Aabb set into two
  /// subsets arbitrarily.
  enable_fallback_split: bool,
}

impl<const D: usize> CenterDataSplitter<D> {
  pub fn new(enable_fallback_split: bool) -> Self {
    Self {
      enable_fallback_split,
    }
  }
}

fn split_indices_wrt_dim<'a, BV>(
  indices: &'a mut [usize],
  aabbs: &[BV],
  split_point: &CenterOf<BV>,
  dim: usize,
  enable_fallback_split: bool,
) -> (&'a mut [usize], &'a mut [usize])
where
  BV: CenterAblePrimitive,
  CenterOf<BV>: std::ops::Index<usize, Output = f32>,
{
  let mut icurr = 0;
  let mut ilast = indices.len();

  // The loop condition we can just do 0..indices.len()
  // instead of the test icurr < ilast because we know
  // we will iterate exactly once per index.
  for _ in 0..indices.len() {
    let i = indices[icurr];
    let center = aabbs[i].get_center();

    if center[dim] > split_point[dim] {
      ilast -= 1;
      indices.swap(icurr, ilast);
    } else {
      icurr += 1;
    }
  }

  if enable_fallback_split && (icurr == 0 || icurr == indices.len()) {
    // We don't want to return one empty set. But
    // this can happen if all the coordinates along the
    // given dimension are equal.
    // In this is the case, we just split in the middle.
    let half = indices.len() / 2;
    indices.split_at_mut(half)
  } else {
    indices.split_at_mut(icurr)
  }
}

impl<BV, const D: usize> QbvhDataSplitter<BV> for CenterDataSplitter<D>
where
  BV: CenterAblePrimitive<Center = Vec3<f32>>,
{
  fn split_dataset<'idx>(
    &mut self,
    indices: &'idx mut [usize],
    aabbs: &[BV],
  ) -> [&'idx mut [usize]; 4] {
    // Compute the center and variance along each dimension.
    // In 3D we compute the variance to not-subdivide the dimension with lowest variance.
    // Therefore variance computation is not needed in 2D because we only have 2 dimension
    // to split in the first place.
    let mut center = CenterOf::<BV>::zero();
    let center_denom = 1.0 / (indices.len() as f32);

    for i in &*indices {
      let coords = aabbs[*i].get_center();
      center = center + coords * center_denom;
    }

    let mut subdiv_dims = [0, 1];

    // Find the axis with minimum variance. This is the axis along
    // which we are **not** subdividing our set. Not needed in 2D.
    if D == 3 {
      let mut vary_x = 0_f32;
      let mut vary_y = 0_f32;
      let mut vary_z = 0_f32;
      let variance_denom = 1.0 / ((indices.len() - 1) as f32);

      for i in &*indices {
        let dir_to_center = aabbs[*i].get_center() - center;
        vary_x += dir_to_center[0] * dir_to_center[0] * variance_denom;
        vary_y += dir_to_center[1] * dir_to_center[1] * variance_denom;
        vary_z += dir_to_center[2] * dir_to_center[2] * variance_denom;
      }
      let (min, min_i) = if vary_x < vary_y {
        (vary_x, 0)
      } else {
        (vary_y, 1)
      };
      let min = if min < vary_z { min_i } else { 2 };

      subdiv_dims[0] = (min + 1) % 3;
      subdiv_dims[1] = (min + 2) % 3;
    }

    // TODO: should we split wrt. the median instead of the average?
    // TODO: we should ensure each subslice contains at least 4 elements each (or less if
    // indices has less than 16 elements in the first place).
    let (left, right) = split_indices_wrt_dim(
      indices,
      aabbs,
      &center,
      subdiv_dims[0],
      self.enable_fallback_split,
    );

    let (left_bottom, left_top) = split_indices_wrt_dim(
      left,
      aabbs,
      &center,
      subdiv_dims[1],
      self.enable_fallback_split,
    );
    let (right_bottom, right_top) = split_indices_wrt_dim(
      right,
      aabbs,
      &center,
      subdiv_dims[1],
      self.enable_fallback_split,
    );
    [left_bottom, left_top, right_bottom, right_top]
  }
}
