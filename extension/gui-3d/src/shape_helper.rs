use crate::*;

pub struct ArrowShape {
  pub body_height: f32,
  pub body_radius: f32,
  pub tip_height: f32,
  pub tip_radius: f32,
  pub segments: usize,
}

impl Default for ArrowShape {
  fn default() -> Self {
    Self {
      body_height: 2.0,
      body_radius: 0.02,
      tip_height: 0.2,
      tip_radius: 0.06,
      segments: 10,
    }
  }
}

impl ArrowShape {
  pub fn build(&self) -> AttributesMeshData {
    let config = TessellationConfig {
      u: 1,
      v: self.segments,
    };
    build_attributes_mesh(|builder| {
      builder
        // body
        .triangulate_parametric(
          &CylinderMeshParameter {
            radius_top: self.body_radius,
            radius_bottom: self.body_radius,
            height: self.body_height,
            ..Default::default()
          }
          .body_surface(),
          config,
          true,
        )
        // tip
        .triangulate_parametric(
          &CylinderMeshParameter {
            radius_top: 0.0,
            radius_bottom: self.tip_radius,
            height: self.tip_height,
            ..Default::default()
          }
          .body_surface()
          .transform3d_by(Mat4::translate((0., self.body_height, 0.))),
          config,
          true,
        );
    })
  }
}
