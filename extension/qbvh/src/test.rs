use std::borrow::Cow;

use colored::Colorize;
use ptree::*;

use crate::*;

mod test_core {

  use arena::{Arena, Handle};
  use rand::{rngs::StdRng, Rng, SeedableRng};
  use rendiation_geometry::HyperAABB;

  use crate::{test::QbvhTreeIterator, *};

  impl<T> IndexedData for Handle<T> {
    fn default() -> Self {
      Self::from_raw_parts(0, 0)
    }

    fn index(&self) -> usize {
      Self::index(self.to_owned())
    }
  }

  #[test]
  fn test_case_qbvh_1() {
    test_qbvh_random_operations(0x23fc68663e15e9e2, 100, Some(51), 0.55);
  }

  #[test]
  fn test_case_qbvh_2() {
    test_qbvh_random_operations(0xd758a3c214dc3866, 2_406, Some(1000), 0.55);
  }

  #[test]
  fn test_case_qbvh_3() {
    test_qbvh_random_operations(0x93bcaea3b92a9bfe, 100, None, 0.53);
  }

  fn test_qbvh_random_operations(
    seed: u64,
    num_iteration: usize,
    debug_specific_iteration: Option<usize>,
    grow_ratio: f32,
  ) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut qbvh = QbvhTester::new();
    let mut added_aabb_handle = vec![];

    for count in 0..num_iteration {
      let x = rng.random_range(0.0..1.0);
      let print_debug =
        debug_specific_iteration.is_some() && count == debug_specific_iteration.unwrap();
      if x < grow_ratio {
        let aabb = generate_random_aabb(&mut rng);
        let handle = qbvh.add_aabb(aabb, print_debug);
        added_aabb_handle.push(handle);
        qbvh.check_topology();
      } else {
        // remove aabb
        if added_aabb_handle.is_empty() {
          continue;
        }
        let aabb_handle =
          added_aabb_handle.swap_remove(rng.random_range(0..added_aabb_handle.len()));
        qbvh.remove_aabb(aabb_handle, print_debug);
        qbvh.check_topology();
      }

      if print_debug {
        let cloned = qbvh.clear_and_rebuild();
        cloned.check_topology();
        println!("rebuild");
        cloned.print_tree();
      }
    }
  }

  pub fn generate_random_aabb(rng: &mut StdRng) -> Box3ForSimd {
    let min_x = rng.random_range(-100..100);
    let min_y = rng.random_range(-100..100);
    let min_z = rng.random_range(-100..100);

    let max_x = rng.random_range(min_x..(min_x + 50));
    let max_y = rng.random_range(min_y..(min_y + 50));
    let max_z = rng.random_range(min_z..(min_z + 50));

    let mins = vec3(min_x as f32, min_y as f32, min_z as f32);
    let maxs = vec3(max_x as f32, max_y as f32, max_z as f32);
    box3_to_box3_for_simd(HyperAABB::new(mins, maxs))
  }

  struct QbvhTester {
    qbvh: Qbvh<Handle<Box3ForSimd>, Box3ForSimd, SimdBox3>,
    workspace: QbvhUpdateWorkspace<Box3ForSimd>,
    aabbs: Arena<Box3ForSimd>,
  }

  impl QbvhTester {
    fn new() -> Self {
      Self {
        qbvh: Qbvh::default(),
        workspace: QbvhUpdateWorkspace::default(),
        aabbs: Arena::new(),
      }
    }

    fn clear_and_rebuild(&self) -> QbvhTester {
      let mut qbvh = Qbvh::<Handle<Box3ForSimd>, Box3ForSimd, SimdBox3>::default();
      let workspace = QbvhUpdateWorkspace::default();
      let aabbs = self.aabbs.clone();

      qbvh.clear_and_rebuild(
        aabbs.iter().map(|(index, aabb)| (index, *aabb)),
        CenterDataSplitter::<3>::new(true),
      );
      QbvhTester {
        qbvh,
        workspace,
        aabbs,
      }
    }

    fn add_aabb(&mut self, aabb: Box3ForSimd, print_debug: bool) -> Handle<Box3ForSimd> {
      if print_debug {
        println!("before insert");
        self.print_tree();
      }
      let index = self.aabbs.insert(aabb);
      self.qbvh.pre_update_bounding_or_insert(index);
      if print_debug {
        println!("before refit");
        self.print_tree();
      }
      let _count = self.qbvh.refit_bounding(|index| self.aabbs[*index]);
      if print_debug {
        println!("before rebalance");
        self.print_tree();
      }
      self
        .qbvh
        .rebalance(&mut self.workspace, CenterDataSplitter::<3>::new(true));
      if print_debug {
        println!("after rebalance");
        self.print_tree();
      }
      index
    }

    fn remove_aabb(&mut self, handle: Handle<Box3ForSimd>, print_debug: bool) {
      if print_debug {
        println!("before remove");
        self.print_tree();
      }
      let _aabb = self.aabbs.remove(handle);
      let _removed = self.qbvh.remove(handle);
      if print_debug {
        println!("before refit");
        self.print_tree();
      }
      let _count = self.qbvh.refit_bounding(|index| self.aabbs[*index]);
      if print_debug {
        println!("before rebalance");
        self.print_tree();
      }
      self
        .qbvh
        .rebalance(&mut self.workspace, CenterDataSplitter::<3>::new(true));
      if print_debug {
        println!("after rebalance");
        self.print_tree();
      }
    }

    fn check_topology(&self) {
      self.qbvh.check_topology();
    }

    fn print_tree(&self) {
      let mut string = vec![];
      let _ = ptree::write_tree(&QbvhTreeIterator::new(&self.qbvh), &mut string);
      println!("{}", String::from_utf8(string).unwrap());
    }
  }
}

impl<'q, LeafData: IndexedData> ptree::TreeItem for QbvhTreeIterator<'q, LeafData> {
  type Child = Self;

  fn write_self<W: std::io::Write>(&self, f: &mut W, _style: &Style) -> std::io::Result<()> {
    let string = format!(
      "{}{} [{:>4.0}, {:>4.0}, {:>4.0}] -> [{:>4.0}, {:>4.0}, {:>4.0}]",
      if self.child_of_leaf { "*" } else { " " },
      self.node,
      self.aabb.min.x,
      self.aabb.min.y,
      self.aabb.min.z,
      self.aabb.max.x,
      self.aabb.max.y,
      self.aabb.max.z,
    );
    let leaf_node = self.qbvh.nodes.get(self.node as usize);
    let is_dirty = !self.child_of_leaf && leaf_node.unwrap().is_dirty();
    let is_changed = !self.child_of_leaf && leaf_node.unwrap().is_changed();

    let colored = if is_dirty {
      string.as_str().red()
    } else if is_changed {
      string.as_str().yellow()
    } else {
      string.as_str().green()
    };
    writeln!(f, "{colored}")
  }

  fn children(&self) -> Cow<'_, [Self::Child]> {
    self.get_children_node()
  }
}

type Box3 = HyperAABBForSimd<Vec3ForSimd<f32>>;

#[derive(Clone)]
pub struct QbvhTreeIterator<'q, LeafData> {
  qbvh: &'q Qbvh<LeafData, Box3, SimdBox3>,
  node: u32,
  aabb: Box3,
  child_of_leaf: bool,
}

impl<'q, LeafData: IndexedData> QbvhTreeIterator<'q, LeafData> {
  pub fn new(qbvh: &'q Qbvh<LeafData, Box3, SimdBox3>) -> Self {
    let aabb = *qbvh.root_aabb();
    let child_of_leaf = qbvh.nodes.is_empty();

    Self {
      qbvh,
      node: 0,
      aabb,
      child_of_leaf,
    }
  }

  fn get_children_node(&self) -> Cow<'_, [Self]> {
    if self.child_of_leaf {
      return Cow::Borrowed(&[]);
    }

    if let Some(node) = self.qbvh.nodes.get(self.node as usize) {
      let self_leaf = node.is_leaf();

      node
        .children
        .iter()
        .enumerate()
        .filter_map(|(lane, &child)| {
          if child != u32::MAX {
            let simd_aabb = node.simd_aabb;
            let aabb = simd_aabb.extract(lane);
            Some(Self {
              qbvh: self.qbvh,
              node: child,
              aabb,
              child_of_leaf: self_leaf,
            })
          } else {
            None
          }
        })
        .collect()
    } else {
      panic!("Unknown node index")
    }
  }
}
