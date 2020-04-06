use super::{HalfEdge, HalfEdgeFace, HalfEdgeVertex};
use rendiation_math::Vec3;
use std::collections::HashMap;

pub struct HalfEdgeMesh<T = PositionNormalVertexData> {
  pub edges: Vec<*mut HalfEdge<T>>,
  pub faces: Vec<*mut HalfEdgeFace<T>>,
  pub vertices: Vec<*mut HalfEdgeVertex<T>>,
}

impl<T> HalfEdgeMesh<T> {
  pub fn remove_face(&mut self, face_id: usize) {
    assert!(face_id < self.faces.len());
    let face = unsafe { &mut *self.faces[face_id] };
    face.visit_around_edge_mut(|_| {})
  }
}

pub struct PositionNormalVertexData {
  pub positions: Vec3<f32>,
  pub normal: Vec3<f32>,
}

impl HalfEdgeMesh {
  pub fn from_geometry(positions: &Vec<f32>, indices: &Vec<u32>) -> Self {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut edges = Vec::new();

    let mut edge_pairs = EdgePairFinder::new();

    for v in 0..positions.len() / 3 {
      let vert = HalfEdgeVertex::new(
        PositionNormalVertexData {
          positions: Vec3::new(positions[3 * v], positions[3 * v + 1], positions[3 * v + 2]),
          normal: Vec3::new(1.0, 0.0, 0.0),
        },
        vertices.len(),
      );
      let vert = Box::into_raw(Box::new(vert));
      vertices.push(vert);
    }

    for f in 0..indices.len() / 3 {
      let face = HalfEdgeFace::new_tri(
        vertices[indices[3 * f] as usize],
        vertices[indices[3 * f + 1] as usize],
        vertices[indices[3 * f + 2] as usize],
        &mut edges,
        &mut edge_pairs,
        vertices.len(),
      );
      faces.push(Box::into_raw(Box::new(face)));
    }

    edge_pairs.find_edge_pairs(&mut edges);

    Self {
      edges,
      faces,
      vertices,
    }
  }
}

impl<T> Drop for HalfEdgeMesh<T> {
  fn drop(&mut self) {
    println!("drop");
    for v in &self.vertices {
      unsafe {
        let _ = Box::from_raw(*v);
      }
    }
    for v in &self.faces {
      unsafe {
        let _ = Box::from_raw(*v);
      }
    }
    for v in &self.edges {
      unsafe {
        let _ = Box::from_raw(*v);
      }
    }
  }
}

pub struct EdgePairFinder<T>(
  HashMap<(*mut HalfEdgeVertex<T>, *mut HalfEdgeVertex<T>), *mut HalfEdge<T>>,
);

impl<T> EdgePairFinder<T> {
  pub fn new() -> Self {
    EdgePairFinder(HashMap::new())
  }
  pub fn insert(
    &mut self,
    k: (*mut HalfEdgeVertex<T>, *mut HalfEdgeVertex<T>),
    v: *mut HalfEdge<T>,
  ) {
    if let Some(_) = self.0.insert(k, v) {
      panic!("not support none manifold geometry")
    }
  }

  pub fn find_edge_pairs(&self, edges: &mut Vec<*mut HalfEdge<T>>) {
    unsafe {
      for edge in edges {
        let edge = &mut **edge;
        if edge.pair_mut().is_none() {
          let key = (
            edge.next_mut().vert_mut() as *mut HalfEdgeVertex<T>,
            edge.vert_mut() as *mut HalfEdgeVertex<T>,
          );
          if let Some(pair) = self.0.get(&key) {
            edge.pair = *pair as *mut HalfEdge<T>;
          }
        }
      }
    }
  }
}
