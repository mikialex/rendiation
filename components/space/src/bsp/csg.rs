use rendiation_algebra::*;
use rendiation_geometry::Plane;

use crate::*;

#[derive(Clone, Default)]
struct CSGNode {
  polygons: Vec<Polygon>,
  plane: Option<Plane>,
  front: Option<Box<CSGNode>>,
  back: Option<Box<CSGNode>>,
}

impl AbstractTree for CSGNode {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    if let Some(front) = &self.front {
      visitor(front)
    }
    if let Some(back) = &self.back {
      visitor(back)
    }
  }
}

#[derive(Clone)]
pub struct CSGMesh {
  polygons: Vec<Polygon>,
}

impl CSGMesh {
  fn from_polygons(polygons: Vec<Polygon>) -> Self {
    Self { polygons }
  }

  pub fn union(&self, other: Self) -> Self {
    let mut a = CSGNode::from_polygons(self.polygons.clone());
    let mut b = CSGNode::from_polygons(other.polygons.clone());
    a.clip_to(&b);
    b.clip_to(&a);
    b.invert();
    b.clip_to(&a);
    b.invert();
    a.build(b.all_polygons());
    return Self::from_polygons(a.all_polygons());
  }
  pub fn subtract(&self, other: Self) -> Self {
    let mut a = CSGNode::from_polygons(self.polygons.clone());
    let mut b = CSGNode::from_polygons(other.polygons.clone());
    a.invert();
    a.clip_to(&b);
    b.clip_to(&a);
    b.invert();
    b.clip_to(&a);
    b.invert();
    a.build(b.all_polygons());
    a.invert();
    return Self::from_polygons(a.all_polygons());
  }
  pub fn intersect(&self, other: Self) -> Self {
    let mut a = CSGNode::from_polygons(self.polygons.clone());
    let mut b = CSGNode::from_polygons(other.polygons.clone());
    a.invert();
    b.clip_to(&a);
    b.invert();
    a.clip_to(&b);
    b.clip_to(&a);
    a.build(b.all_polygons());
    a.invert();
    return Self::from_polygons(a.all_polygons());
  }

  pub fn inverse(&self) -> Self {
    let mut csg = self.clone();
    csg.polygons.iter_mut().for_each(|p| {
      p.flip();
    });
    return csg;
  }
}

impl CSGNode {
  /// Convert solid space to empty space and empty space to solid space.
  fn invert(&mut self) {
    for polygon in &mut self.polygons {
      polygon.flip();
    }
    if let Some(plane) = &mut self.plane {
      plane.flip();
    }
    if let Some(front) = &mut self.front {
      front.invert();
    }
    if let Some(back) = &mut self.back {
      back.invert();
    }
    std::mem::swap(&mut self.front, &mut self.back);
  }

  /// Recursively remove all polygons in `polygons` that are inside this BSP tree.
  fn clip_polygons(&self, polygons: &Vec<Polygon>) -> Vec<Polygon> {
    if let Some(plane) = &self.plane {
      let mut front = Vec::new();
      let mut back = Vec::new();

      for polygon in &self.polygons {
        polygon.split(*plane, &mut front, &mut back, &mut front, &mut back);
      }

      let mut result = Vec::new();

      if let Some(front_node) = &self.front {
        result.extend(front_node.clip_polygons(&front));
      }
      if let Some(back_node) = &self.back {
        result.extend(back_node.clip_polygons(&back));
      }
      result
    } else {
      polygons.clone()
    }
  }

  /// Remove all polygons in this BSP tree that are inside the other BSP tree `bsp`.
  fn clip_to(&mut self, bsp: &Self) {
    self.traverse(&mut |n| n.polygons = bsp.clip_polygons(&n.polygons));
  }

  /// Return a list of all polygons in this BSP tree.
  fn all_polygons(&self) -> Vec<Polygon> {
    let mut result = Vec::new();
    self.traverse(&mut |n| result.extend(n.polygons.iter().cloned()));
    result
  }

  /// Build a BSP tree out of `polygons`. When called on an existing tree, the
  /// new polygons are filtered down to the bottom of the tree and become new
  /// nodes there. Each set of polygons is partitioned using the first polygon
  /// (no heuristic is used to pick a good split).
  fn build(&mut self, polygons: Vec<Polygon>) {
    if polygons.is_empty() {
      return;
    }

    let plane = self.plane.get_or_insert_with(|| polygons[0].plane);

    let mut front = Vec::new();
    let mut back = Vec::new();

    for polygon in polygons {
      polygon.split(*plane, &mut polygons, &mut polygons, &mut front, &mut back);
    }
    self.front.get_or_insert_default().build(front);
    self.back.get_or_insert_default().build(back);
  }

  fn from_polygons(polygons: Vec<Polygon>) -> Self {
    let mut node = Self::default();
    node.build(polygons);
    node
  }
}

#[derive(Clone, Copy)]
struct Vertex {
  position: Vec3<f32>,
  normal: Vec3<f32>,
  uv: Vec3<f32>,
}

impl Vertex {
  fn flip(&mut self) {
    self.normal = self.normal.reverse();
  }
}

/// Represents a convex polygon. The vertices used to initialize a polygon must
/// be coplanar and form a convex loop. They do not have to be `Vertex`
/// instances but they must behave similarly (duck typing can be used for
/// customization).
#[derive(Clone)]
struct Polygon {
  plane: Plane,
  vertices: Vec<Vertex>,
}

const COPLANAR: u8 = 0;
const FRONT: u8 = 1;
const BACK: u8 = 2;
const SPANNING: u8 = 3;
const EPSILON: f32 = 1e-5;

enum PlaneSideType {
  Front,
  Back,
  Coplanar,
  Spanning,
}

impl Polygon {
  fn flip(&mut self) {
    for v in &mut self.vertices {
      v.flip()
    }
    self.plane.flip()
  }

  /// Split `polygon` by this plane if needed, then put the polygon or polygon
  /// fragments in the appropriate lists. Coplanar polygons go into either
  /// `coplanarFront` or `coplanarBack` depending on their orientation with
  /// respect to this plane. Polygons in front or in back of this plane go into
  /// either `front` or `back`.
  fn split(
    &self,
    plane: Plane,
    coplanar_front: &mut Vec<Polygon>,
    coplanar_back: &mut Vec<Polygon>,
    front: &mut Vec<Polygon>,
    back: &mut Vec<Polygon>,
  ) {
    // Classify each point as well as the entire polygon into one of the above
    // four classes.
    let mut polygonType = 0;
    let mut types = Vec::new();
    for vertex in &self.vertices {
      let t = plane.normal.dot(vertex.position) - plane.constant;
      let ty = if t < -EPSILON {
        BACK
      } else if t > EPSILON {
        FRONT
      } else {
        COPLANAR
      };
      polygonType |= ty;
      types.push(ty);
    }

    // Put the polygon in the correct list, splitting it when necessary.
    match polygonType {
      COPLANAR => if plane.normal.dot(self.plane.normal) > 0. {
        coplanar_front
      } else {
        coplanar_back
      }
      .push(self.clone()),

      FRONT => front.push(self.clone()),

      BACK => back.push(self.clone()),

      SPANNING => {
        let mut f = Vec::new();
        let mut b = Vec::new();
        for i in 0..self.vertices.len() {
          let j = (i + 1) % self.vertices.len();
          let ti = types[i];
          let tj = types[j];
          let vi = self.vertices[i];
          let vj = self.vertices[j];
          if ti != BACK {
            f.push(vi);
          }
          if ti != FRONT {
            b.push(if ti != BACK { vi.clone() } else { vi });
          }
          if (ti | tj) == SPANNING {
            // let t = (plane.constant - plane.normal.dot(vi.position)) / plane.normal.dot(vj.position -vi.position);
            // let v = vi.interpolate(vj, t);
            // f.push(v);
            // b.push(v.clone());
          }
        }
        if f.len() >= 3 {
          // front.push(new Polygon(f, polygon.shared));
        }
        if b.len() >= 3 {
          // back.push(new Polygon(b, polygon.shared));
        }
      }
      _ => unreachable!(),
    }
  }
}
