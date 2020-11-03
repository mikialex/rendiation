pub use nyxt_core::*;

use space_indexer::{
  bvh::BalanceTree,
  bvh::{test::bvh_build, SAH},
  utils::generate_boxes_in_space,
  utils::TreeBuildOption,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test_bvh() {
  let boxes = generate_boxes_in_space(20000, 10000., 1.);

  for _ in 0..10 {
    let _ = bvh_build(
      &boxes,
      &mut BalanceTree,
      &TreeBuildOption {
        max_tree_depth: 15,
        bin_size: 10,
      },
    );
  }

  let mut sah = SAH::new(4);
  for _ in 0..10 {
    let _ = bvh_build(
      &boxes,
      &mut sah,
      &TreeBuildOption {
        max_tree_depth: 15,
        bin_size: 10,
      },
    );
  }
}

pub use rendiation_shader_library::fog::*;
