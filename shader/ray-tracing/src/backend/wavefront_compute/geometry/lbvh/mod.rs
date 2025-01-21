use std::sync::atomic::{AtomicBool, Ordering};

use bytemuck::Zeroable;
use rendiation_algebra::{vec2, vec3, RealVector, Vec2, Vec3, Vector};
use rendiation_geometry::Box3;

use crate::DeviceBVHNode;

// https://developer.nvidia.com/blog/thinking-parallel-part-iii-tree-construction-gpu/
// Calculates a 30-bit Morton code for the
// given 3D point located within the unit cube [0,1].
fn morton_3d_cpu(normalized_xyz: Vec3<f32>) -> u32 {
  // Expands a 10-bit integer into 30 bits
  // by inserting 2 zeros after each bit.
  fn expand_bits(v: u32) -> u32 {
    let v = (v.overflowing_mul(0x00010001).0) & 0xFF0000FF;
    let v = (v.overflowing_mul(0x00000101).0) & 0x0F00F00F;
    let v = (v.overflowing_mul(0x00000011).0) & 0xC30C30C3;
    let v = (v.overflowing_mul(0x00000005).0) & 0x49249249;
    #[allow(clippy::let_and_return)]
    v
  }

  let xyz = (normalized_xyz * Vec3::splat(1024.))
    .max(Vec3::zero())
    .min(Vec3::splat(1023.));
  let x = expand_bits(xyz.x() as u32);
  let y = expand_bits(xyz.y() as u32);
  let z = expand_bits(xyz.z() as u32);
  x * 4 + y * 2 + z
}

// // Calculates a 30-bit Morton code for the
// // given 3D point located within the unit cube [0,1].
// fn morton_3d_gpu(normalized_xyz: Node<Vec3<f32>>) -> Node<u32> {
//   // Expands a 10-bit integer into 30 bits
//   // by inserting 2 zeros after each bit.
//   fn expand_bits(v: Node<u32>) -> Node<u32> {
//     let v = (v * val(0x00010001)) & val(0xFF0000FF);
//     let v = (v * val(0x00000101)) & val(0x0F00F00F);
//     let v = (v * val(0x00000011)) & val(0xC30C30C3);
//     let v = (v * val(0x00000005)) & val(0x49249249);
//     #[allow(clippy::let_and_return)]
//     v
//   }
//
//   let xyz = (normalized_xyz * val(Vec3::splat(1024.))).clamp(Vec3::zero(), Vec3::splat(1023.));
//   let bits = xyz.into_u32();
//   let x = expand_bits(bits.x());
//   let y = expand_bits(bits.y());
//   let z = expand_bits(bits.z());
//   x * val(4) + y * val(2) + z
// }

/// sort center and source idx in-place
/// points: [(morton, idx)]
fn sort_morton_cpu(morton_idx: &mut [Vec2<u32>]) {
  morton_idx.sort_by_cached_key(|i| i.x);
}

// todo support duplicated morton code:
// 1. check let same = code[idx] == code[idx-1] for idx = 0 to m
// 2. compute prefix sum of 'same' as new index (last value as n)
// 3. allocate redirect[n] {left, right} inclusive
// 4. write left bound if code[idx-1] < code[idx], right if code[idx] < code[idx+1] for idx = 0 to m

/// generate triangle centers and global bounding
fn triangles_to_bounding_cpu(vertices: &[f32], indices: &[u32]) -> (Vec<Box3>, Box3) {
  assert_eq!(indices.len() % 3, 0);
  let index_len = indices.len() / 3;
  fn read_vec3<T: Copy>(array: &[T], i: usize) -> Vec3<T> {
    vec3(array[i * 3], array[i * 3 + 1], array[i * 3 + 2])
  }
  let mut boxes = vec![];
  let mut global = Box3::default();
  for i in 0..index_len {
    let abc = read_vec3(indices, i);
    let a = read_vec3(vertices, abc.x as usize);
    let b = read_vec3(vertices, abc.y as usize);
    let c = read_vec3(vertices, abc.z as usize);
    let mut local = Box3::default();
    local.expand_by_point(a);
    local.expand_by_point(b);
    local.expand_by_point(c);
    // let center = local.center();
    boxes.push(local);
    global.expand_by_other(local);
    // global.expand_by_point(center);
  }
  (boxes, global)
}

/// returns [(morton, idx)]
fn centers_to_morton_cpu(boxes: &[Box3], global_bounding: Box3) -> Vec<Vec2<u32>> {
  let size = global_bounding.size();

  boxes
    .iter()
    .enumerate()
    .map(|(i, box3)| {
      let normalized = (box3.center() - global_bounding.min) / size;
      let morton_code = morton_3d_cpu(normalized);
      vec2(morton_code, i as u32)
    })
    .collect()
}

/// is_leaf === (left == right) === (id != u32::MAX)
/// parent of root === 0
/// has left child === has right child === (left != u32::MAX)
#[derive(Default, Copy, Clone, Debug)]
struct LbvhNode {
  parent: u32,
  left: u32,
  right: u32,
  id: u32,
}
fn lbvh_tree_cpu(morton: &[Vec2<u32>]) -> Vec<LbvhNode> {
  fn distance(values: &[u32], ia: i32, ib: i32) -> i32 {
    if ia == ib {
      println!("!");
    }
    let size = values.len() as i32;
    if ib < 0 || ib >= size {
      return -1;
    }
    let a = values[ia as usize];
    let b = values[ib as usize];
    (a ^ b).leading_zeros() as i32
  }

  let len = morton.len();
  let idx: Vec<_> = morton.iter().map(|v| v.y).collect();
  let v: Vec<_> = morton.iter().map(|v| v.x).collect();
  let v = &v;

  // n-1 nodes and n leafs
  let mut nodes = vec![
    LbvhNode {
      parent: u32::MAX,
      left: u32::MAX,
      right: u32::MAX,
      id: u32::MAX,
    };
    2 * len - 1
  ];
  let leaf_offset = len - 1;
  for i in 0..len {
    nodes[i + leaf_offset].id = idx[i];
  }

  for i in 0..(len - 1) as i32 {
    // println!("node {i}");
    let d = (distance(v, i, i + 1) - distance(v, i, i - 1)).signum();
    let d_min = distance(v, i, i - d);
    let mut l_max = 2;
    while distance(v, i, i + l_max * d) > d_min {
      l_max *= 2;
    }
    // println!("  dir {d}, l_max {l_max}");

    let mut t = l_max >> 1;
    let mut l = 0;
    while t > 0 {
      if distance(v, i, i + (l + t) * d) > d_min {
        l += t;
      }
      t >>= 1;
    }
    let j = i + l * d;

    let d_node = distance(v, i, j);
    // println!("  range {}, {}, d_node {}", i.min(j), i.max(j), 32 - d_node);
    let mut s = 0;
    let mut digit = 0;
    let mut t = ((l >> digit) & 1) + (l >> (digit + 1));
    while t >= 1 {
      if distance(v, i, i + (s + t) * d) > d_node {
        s += t;
      }
      digit += 1;
      t = ((l >> digit) & 1) + (l >> (digit + 1));
    }
    let mid = i + s * d + d.min(0);

    let left = if i.min(j) == mid {
      leaf_offset as u32 + mid as u32
    } else {
      mid as u32
    };
    let right = if i.max(j) == mid + 1 {
      leaf_offset as u32 + (mid + 1) as u32
    } else {
      (mid + 1) as u32
    };
    // println!("  children {left:?}, {mid}, {right:?}");
    nodes[i as usize].id = u32::MAX;
    nodes[i as usize].left = left;
    nodes[i as usize].right = right;
    nodes[left as usize].parent = i as u32;
    nodes[right as usize].parent = i as u32;
  }

  // println!("children {:?}", nodes);
  nodes
}

fn lbvh_nodes_to_device_nodes(lbvh_nodes: &[LbvhNode], boxes: &[Box3]) -> Vec<DeviceBVHNode> {
  assert_eq!(boxes.len() * 2 - 1, lbvh_nodes.len());
  let leaf_node_start = boxes.len() - 1;
  let n = boxes.len();

  let mut nodes = vec![DeviceBVHNode::zeroed(); lbvh_nodes.len()];

  for node in &mut nodes {
    node.hit_next = u32::MAX;
    node.miss_next = u32::MAX;
    node.content_range = vec2(u32::MAX, u32::MAX);
  }

  // for each leaf: aabb, range (hit_next & miss_next not set)
  for i in 0..n {
    let src = lbvh_nodes[i + n - 1];
    let dst = &mut nodes[i + n - 1];
    dst.content_range = vec2(src.id, src.id + 1); // todo node with same morton code
    dst.aabb_min = boxes[src.id as usize].min;
    dst.aabb_max = boxes[src.id as usize].max;
  }

  // for internal nodes, top-down search for self
  for i in 0..(n - 1) {
    let mut curr_node = 0;
    let mut curr_miss = u32::MAX;
    let mut src = lbvh_nodes[curr_node as usize];

    // find self
    while curr_node != u32::MAX {
      src = lbvh_nodes[curr_node as usize];
      if curr_node != i as u32 {
        // search self

        if (i as u32) < src.right {
          // go left, miss = right
          curr_miss = src.right;
          curr_node = src.left;
        } else {
          // go right, no change to miss
          curr_node = src.right;
        }
      } else {
        break;
      }
    }

    // found self, set hit_next & miss_next
    if curr_node < n as u32 {
      nodes[curr_node as usize].hit_next = src.left;
      nodes[curr_node as usize].miss_next = curr_miss;
    }

    // set leaf node hit_next & miss_next
    if src.left >= leaf_node_start as u32 {
      nodes[src.left as usize].hit_next = src.right;
      nodes[src.left as usize].miss_next = src.right;
    }
    if src.right >= leaf_node_start as u32 {
      nodes[src.right as usize].hit_next = curr_miss;
      nodes[src.right as usize].miss_next = curr_miss;
    }
  }

  nodes
}

fn device_nodes_merge_aabb(
  mut nodes: Vec<DeviceBVHNode>,
  lbvh_nodes: &[LbvhNode],
) -> Vec<DeviceBVHNode> {
  let n = (nodes.len() + 1) / 2;
  // atomics for aabb generation
  let visited = (0..(n - 1))
    .map(|_| AtomicBool::new(false))
    .collect::<Vec<_>>();

  // all leaf nodes
  for i in 0..n {
    let mut k = i + n - 1;

    loop {
      let parent = lbvh_nodes[k].parent;
      if parent == u32::MAX {
        break;
      }
      let parent = parent as usize;

      let result =
        visited[parent].compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed);
      match result {
        Ok(_) => {
          // first to visit parent: set aabb, break
          nodes[parent].aabb_min = nodes[k].aabb_min;
          nodes[parent].aabb_max = nodes[k].aabb_max;
          // println!("set {parent} aabb from {k}");
          break;
        }
        Err(_) => {
          // second to visit parent: merge aabb, continue
          nodes[parent].aabb_min = nodes[parent].aabb_min.min(nodes[k].aabb_min);
          nodes[parent].aabb_max = nodes[parent].aabb_max.max(nodes[k].aabb_max);
          // println!("merge {parent} aabb with {k}");
          k = parent;
        }
      }
    }
  }

  nodes
}

fn print_device_nodes(nodes: &[DeviceBVHNode]) {
  for (i, node) in nodes.iter().enumerate() {
    if node.content_range.x == u32::MAX {
      println!(
        "internal node {} hit_next = {}, miss_next = {}, box {:?} - {:?}",
        i, node.hit_next, node.miss_next, node.aabb_min, node.aabb_max
      );
    } else {
      println!(
        "leaf node {} hit_next = {}, miss_next = {}, box {:?} - {:?}, id: {}",
        i, node.hit_next, node.miss_next, node.aabb_min, node.aabb_max, node.content_range.x
      );
    }
  }
}

#[test]
fn test_lbvh() {
  let mut morton_idx = vec![
    vec2(0b00001, 0),
    vec2(0b00010, 1),
    vec2(0b00100, 2),
    vec2(0b00101, 3),
    vec2(0b10011, 4),
    vec2(0b11000, 5),
    vec2(0b11001, 6),
    vec2(0b11110, 7),
  ];
  sort_morton_cpu(&mut morton_idx);
  let lbvh_nodes = lbvh_tree_cpu(&morton_idx);

  let device_nodes = lbvh_nodes_to_device_nodes(&lbvh_nodes, vec![Box3::default(); 8].as_slice());
  let device_nodes = device_nodes_merge_aabb(device_nodes, &lbvh_nodes);

  print_device_nodes(&device_nodes);
}

#[test]
fn test_all() {
  let boxes = (0..16)
    .map(|i| {
      let x = (i % 4) as f32;
      let y = (i / 4) as f32;
      Box3::new(vec3(x, y, 0.), vec3(x + 1., y + 1., 1.))
    })
    .collect::<Vec<_>>();

  let mut morton = centers_to_morton_cpu(&boxes, Box3::new(vec3(0., 0., 0.), vec3(4., 4., 1.)));
  sort_morton_cpu(&mut morton);
  let lbvh_nodes = lbvh_tree_cpu(&morton);
  let device_nodes = lbvh_nodes_to_device_nodes(&lbvh_nodes, &boxes);
  let device_nodes = device_nodes_merge_aabb(device_nodes, &lbvh_nodes);

  print_device_nodes(&device_nodes);
}

pub fn build_tlas_bvh_cpu(boxes: &[Box3], node_offset: u32, id_offset: u32) -> Vec<DeviceBVHNode> {
  let global_box = boxes.iter().collect::<Box3>();

  let mut morton = centers_to_morton_cpu(boxes, global_box);
  sort_morton_cpu(&mut morton);
  let lbvh_nodes = lbvh_tree_cpu(&morton);
  let device_nodes = lbvh_nodes_to_device_nodes(&lbvh_nodes, boxes);
  let mut device_nodes = device_nodes_merge_aabb(device_nodes, &lbvh_nodes);

  device_nodes.iter_mut().for_each(|node| {
    if node.content_range.x != u32::MAX {
      node.content_range.x += id_offset;
      node.content_range.y += id_offset;
    }
    if node.hit_next != u32::MAX {
      node.hit_next += node_offset;
    }
    if node.miss_next != u32::MAX {
      node.miss_next += node_offset;
    }
  });

  // print_device_nodes(&device_nodes);

  device_nodes
}

pub fn build_geometry_bvh_cpu(vertices: &[f32], indices: &[u32]) -> (Vec<DeviceBVHNode>, Box3) {
  let (boxes, global_bounding) = triangles_to_bounding_cpu(vertices, indices);
  let mut morton = centers_to_morton_cpu(&boxes, global_bounding);
  sort_morton_cpu(&mut morton);
  let lbvh_nodes = lbvh_tree_cpu(&morton);
  let device_nodes = lbvh_nodes_to_device_nodes(&lbvh_nodes, &boxes);

  let nodes = device_nodes_merge_aabb(device_nodes, &lbvh_nodes);
  (nodes, global_bounding)
}
