use rendiation_math::*;
use rendiation_math_entity::*;
use std::{cmp::Ordering, ops::Range};

struct FlattenBVH {
  nodes: Vec<FlattenBVHNode>,
  sorted_primitive_index: Vec<usize>,
}

impl FlattenBVH {
  pub fn build<T: BVHBuildStrategy>(source: impl FlattenBVHBuildSource) -> Self {
    let items_count = source.get_items_count();
    let mut index_list: Vec<usize> = (0..items_count).map(|x| x).collect();
    let primitives: Vec<BuildPrimitive> = (0..items_count)
      .map(|x| BuildPrimitive::new(source.get_items_bounding_box(x)))
      .collect();

    let root = FlattenBVHNode::new(&primitives, &index_list, 0..items_count, 0);
    let mut nodes = Vec::new();
    nodes.push(root);
    Self {
      nodes,
      sorted_primitive_index: index_list,
    }
  }
}

struct BuildPrimitive {
  bbox: Box3,
  center: Vec3<f32>,
}

impl BuildPrimitive {
  fn new(bbox: Box3) -> Self {
    Self {
      bbox,
      center: bbox.center(),
    }
  }

  fn compare_center(&self, axis: Axis, other: &BuildPrimitive) -> Ordering{
    match axis {
      Axis::X => self.center.x.partial_cmp(&other.center.x).unwrap(),
      Axis::Y => self.center.y.partial_cmp(&other.center.y).unwrap(),
      Axis::Z => self.center.z.partial_cmp(&other.center.z).unwrap(),
    }
  }
}

trait FlattenBVHBuildSource {
  fn get_items_count(&self) -> usize;
  fn get_items_bounding_box(&self, item_index: usize) -> Box3;
}

trait BVHBuildStrategy {
  fn split(
    build_source: &Vec<BuildPrimitive>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode>,
    node: &FlattenBVHNode,
  );
}

struct SAH;

impl BVHBuildStrategy for SAH {
  fn split(
    build_source: &Vec<BuildPrimitive>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode>,
    node: &FlattenBVHNode,
  ) {
    let ranged_index = index_source.get_mut(node.primitive_range.clone()).unwrap();
    let split_axis = node.bbox.longest_axis();
    ranged_index.sort_unstable_by(|a, b|{
      let bp_a = &build_source[*a];
      let bp_b = &build_source[*b];
      bp_a.compare_center(split_axis, bp_b)
    })
    
  }
}

pub struct FlattenBVHNode {
  pub bbox: Box3,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub child: Option<FlattenBVHNodeChildInfo>,
}

impl FlattenBVHNode {
  fn new(
    build_source: &Vec<BuildPrimitive>,
    index_source: &Vec<usize>,
    range: Range<usize>,
    depth: usize,
  ) -> Self {
    let primitive_range = range.clone();
    let ranged_index_source = build_source.get(range).unwrap();
    let bbox = Box3::from_boxes(ranged_index_source.iter().map(|p| p.bbox));
    Self {
      bbox,
      primitive_range,
      depth,
      child: None,
    }
  }
}

pub struct FlattenBVHNodeChildInfo {
  pub left_index: usize,
  pub right_index: usize,
  pub split_axis: Axis,
}

// #[derive(Debug, Clone)]
// pub enum SplitMethod {
//   SAH,
//   Middle,
//   EqualCounts,
// }

// pub struct BVHNode {
//   pub bounding_box: Box3,
//   pub left: Option<Box<BVHNode>>,
//   pub right: Option<Box<BVHNode>>,
//   pub split_axis: Option<Axis>,
//   pub primitive_start: u64,
//   pub primitive_count: u64,
//   pub depth: u64,
// }

// const BVH_MAX_BIN_SIZE: u64 = 1;
// const BVH_MAX_DEPTH: u64 = 10;

// // https://matthias-endler.de/2017/boxes-and-trees/
// impl BVHNode {
//   pub fn build_from_range_primitives(
//     primitive_list: &Vec<Primitive>,
//     start: u64,
//     count: u64,
//   ) -> BVHNode {
//     let bbox = get_range_primitives_bounding(primitive_list, start, count);
//     return BVHNode {
//       bounding_box: bbox,
//       left: None,
//       right: None,
//       split_axis: None,
//       primitive_start: start,
//       primitive_count: count,
//       depth: 0,
//     };
//   }

//   pub fn should_split(&self) -> bool {
//     return self.primitive_count < BVH_MAX_BIN_SIZE || self.depth > BVH_MAX_DEPTH;
//   }

//   pub fn split(&mut self, primtive_list: &mut [Primitive], spliter: &dyn Fn(&mut BVHNode) -> ()) {
//     if !self.should_split() {
//       return;
//     }

//     self.split_axis = Some(self.bounding_box.longest_axis());

//     // TODO opti, maybe we should put this procedure in spliter
//     match self.split_axis {
//       Some(Axis::X) => primtive_list.sort_unstable_by(|a, b| a.cmp_center_x(b)),
//       Some(Axis::Y) => primtive_list.sort_unstable_by(|a, b| a.cmp_center_y(b)),
//       Some(Axis::Z) => primtive_list.sort_unstable_by(|a, b| a.cmp_center_z(b)),
//       None => panic!(""),
//     }

//     spliter(self);

//     match &mut self.left {
//       Some(node) => &node.split(primtive_list, spliter),
//       None => panic!(""),
//     };
//     // self.left.split(primtive_list, spliter);
//   }
// }

// fn build_equal_counts(node: &mut BVHNode) {
//   node.left = None;
//   node.right = None;
// }

// pub struct Primitive {
//   pub bounding_box: Box3,
//   pub center_point: Vec3,
//   pub index: u64,
// }

// impl Primitive {
//   pub fn cmp_center_x(&self, other: &Primitive) -> std::cmp::Ordering {
//     if self.center_point.x < other.center_point.x {
//       std::cmp::Ordering::Less
//     } else {
//       std::cmp::Ordering::Greater
//     }
//   }

//   pub fn cmp_center_y(&self, other: &Primitive) -> std::cmp::Ordering {
//     if self.center_point.y < other.center_point.y {
//       std::cmp::Ordering::Less
//     } else {
//       std::cmp::Ordering::Greater
//     }
//   }

//   pub fn cmp_center_z(&self, other: &Primitive) -> std::cmp::Ordering {
//     if self.center_point.z < other.center_point.z {
//       std::cmp::Ordering::Less
//     } else {
//       std::cmp::Ordering::Greater
//     }
//   }
// }

// pub struct BVHAccel {
//   root: BVHNode,
//   primitives: Vec<Primitive>,
// }

// impl BVHAccel {
//   pub fn build(primitives: Vec<Primitive>) -> BVHAccel {
//     let mut bvh = BVHAccel {
//       root: BVHNode::build_from_range_primitives(&primitives, 0, primitives.len() as u64),
//       primitives,
//     };
//     bvh.root.split(&mut bvh.primitives, &build_equal_counts);
//     bvh
//   }
// }

// fn get_range_primitives_bounding(primitive_list: &Vec<Primitive>, start: u64, count: u64) -> Box3 {
//   let mut bbox = primitive_list[start as usize].bounding_box.clone();
//   for pid in start..(start + count) {
//     bbox.expand_by_box(primitive_list[pid as usize].bounding_box);
//   }
//   bbox
// }
