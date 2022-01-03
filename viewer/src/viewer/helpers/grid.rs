use rendiation_algebra::*;

use crate::{FatLineVertex, SceneNode};

use super::HelperLineMesh;

pub struct GridHelper {
  pub enabled: bool,
  pub root: SceneNode,
  pub config: GridConfig,
}

pub struct GridConfig {
  pub width_segments: usize,
  pub height_segments: usize,
  pub width: usize,
  pub height: usize,
}

fn build_grid(config: GridConfig) -> HelperLineMesh {
  let mut lines = Vec::new();

  for x in 0..config.width_segments {
    for y in 0..config.height_segments {
      let start = Vec3::new(x as f32, 0., y as f32);
      let end = Vec3::new(x as f32, 0., y as f32);

      let line = FatLineVertex {
        start,
        end,
        color: Vec4::splat(1.),
      };
      lines.push(line)
    }
  }

  HelperLineMesh::new(lines)
}
