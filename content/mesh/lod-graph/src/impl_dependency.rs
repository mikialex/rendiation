use crate::*;

pub trait MeshLodGraphBuilder {
  fn simplify(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
    locked_edges: &EdgeFinder,
    config: MeshLODGraphSimplificationConfig,
  ) -> MeshLODGraphSimplificationResult;

  fn segment_triangles(&self, input: &MeshBufferSource) -> SegmentResult;
  fn segment_meshlets(&self, input: &[Meshlet]) -> SegmentResult;
}

pub struct MeshLODGraphSimplificationConfig {
  pub target_tri_num: u32,
  pub tri_num_limit: u32,
  pub target_error: f32,
}

pub struct MeshLODGraphSimplificationResult {
  pub mesh: MeshBufferSource,
  pub error: f32,
}

type TrianglePrimitive = Triangle<Vec3<f32>>;

impl SegmentationSource for MeshBufferSource {
  type Item = TrianglePrimitive;

  fn count(&self) -> u32 {
    (self.indices.len() / 3) as u32
  }

  fn get_item(&self, index: u32) -> Option<Self::Item> {
    let idx = (index as usize) * 3;
    Some(Triangle::new(
      self.vertices[idx].position,
      self.vertices[idx].position,
      self.vertices[idx].position,
    ))
  }
}

struct MeshletSegmentationSource<'a>(&'a [Meshlet]);
impl<'a> SegmentationSource for MeshletSegmentationSource<'a> {
  type Item = Meshlet;

  fn count(&self) -> u32 {
    self.0.len() as u32
  }

  fn get_item(&self, index: u32) -> Option<Self::Item> {
    self.0.get(index as usize).cloned()
  }
}
