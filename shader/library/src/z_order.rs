use crate::*;

/// map 6bit id to 8x8 grid point
#[shader_fn]
pub fn remap_for_wave_reduction(a: Node<u32>) -> Node<Vec2<u32>> {
  let x = a
    .extract_bits(val(2), val(3))
    .insert_bits(a, val(0), val(1));
  let n = a.extract_bits(val(1), val(2));
  let y = a
    .extract_bits(val(3), val(3))
    .insert_bits(n, val(0), val(2));
  (x, y).into()
}

// 8x8 tiles
pub const LAUNCH_ID_TILE_POW_2: u32 = 3;
pub const LAUNCH_ID_TILE_SIZE: u32 = 1 << LAUNCH_ID_TILE_POW_2;
pub const LAUNCH_ID_TILE_MASK: u32 = LAUNCH_ID_TILE_SIZE - 1;
pub const LAUNCH_ID_TILE_AREA: u32 = LAUNCH_ID_TILE_SIZE * LAUNCH_ID_TILE_SIZE;

fn pad_pow2(input: u32, mask: u32) -> u32 {
  (input + mask) & !mask
}
fn pad_pow2_device(input: Node<u32>, mask: u32) -> Node<u32> {
  (input + val(mask)) & val(!mask)
}

pub fn pas_z_order_size(size: (u32, u32)) -> (u32, u32) {
  let w_pad = pad_pow2(size.0, LAUNCH_ID_TILE_MASK);
  let h_pad = pad_pow2(size.1, LAUNCH_ID_TILE_MASK);
  (w_pad, h_pad)
}

#[shader_fn]
pub fn compute_z_order_dispatch_id(
  launch_size: Node<Vec3<u32>>,
  linear_id: Node<u32>,
) -> Node<Vec3<u32>> {
  // todo move to cpu side
  let launch_padded_w = pad_pow2_device(launch_size.x(), LAUNCH_ID_TILE_MASK);
  let launch_padded_h = pad_pow2_device(launch_size.y(), LAUNCH_ID_TILE_MASK);
  let page_size = launch_padded_w * launch_padded_h;

  let z = linear_id / page_size;
  let xy_id = linear_id % page_size;
  let local_id = xy_id % val(LAUNCH_ID_TILE_AREA);
  let tile_id = xy_id / val(LAUNCH_ID_TILE_AREA);
  let tile_w = launch_padded_w >> val(LAUNCH_ID_TILE_POW_2);
  let tile_h = launch_padded_h >> val(LAUNCH_ID_TILE_POW_2);

  let local_xy = remap_for_wave_reduction_fn(local_id);
  let tile_x = tile_id % tile_w;
  let tile_y = tile_id / tile_h;
  let x = tile_x * val(LAUNCH_ID_TILE_SIZE) + local_xy.x();
  let y = tile_y * val(LAUNCH_ID_TILE_SIZE) + local_xy.y();

  (x, y, z).into()
}
