use crate::*;

/// every thing in object space
#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, PartialEq)]
pub struct LODBound {
  pub error: f32,
  pub radius: f32,
  // note, expand vec3 to avoid unnecessary padding cost
  pub x_position: f32,
  pub y_position: f32,
  pub z_position: f32,
}

impl LODBound {
  pub fn new(error: f32, radius: f32, position: Vec3<f32>) -> Self {
    Self {
      error,
      radius,
      x_position: position.x,
      y_position: position.y,
      z_position: position.z,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, PartialEq)]
pub struct LODBoundPair {
  pub self_lod: LODBound,
  pub parent_lod: LODBound,
}

impl LODBoundPair {
  pub fn new(self_lod: LODBound, parent_lod: LODBound) -> Self {
    Self {
      self_lod,
      parent_lod,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
pub struct LODDecider {
  pub camera_projection: Mat4<f32>,
  pub view_size: Vec2<f32>,
  pub camera_near: f32,
  pub camera_world_position: HighPrecisionTranslationUniform,
}

impl LODDeciderShaderAPIInstance {
  pub fn exact_lod_cut(
    &self,
    self_lod: Node<LODBound>,
    parent: Node<LODBound>,
    meshlet_local_to_render: Node<Mat4<f32>>,
    world_position: Node<HighPrecisionTranslation>,
  ) -> Node<bool> {
    // assume 1px to cause visual difference
    let pixel_error_threshold = val(1.);

    let translate_into_render_space = hpt_sub_hpt(
      world_position,
      hpt_uniform_to_hpt(self.camera_world_position),
    );

    let parent_lod_ok = self.lod_error_is_imperceptible(
      parent,
      pixel_error_threshold,
      meshlet_local_to_render,
      translate_into_render_space,
    );
    let self_lod_ok = self.lod_error_is_imperceptible(
      self_lod,
      pixel_error_threshold,
      meshlet_local_to_render,
      translate_into_render_space,
    );

    self_lod_ok.and(parent_lod_ok.not())
  }

  fn lod_error_is_imperceptible(
    &self,
    lod: Node<LODBound>,
    pixel_error_threshold: Node<f32>,
    meshlet_world_mat_no_translation: Node<Mat4<f32>>,
    translate_into_render_space: Node<Vec3<f32>>,
  ) -> Node<bool> {
    let lod = lod.expand();
    let meshlet_bounding_center_in_local: Node<Vec3<f32>> =
      (lod.x_position, lod.y_position, lod.z_position).into();
    let meshlet_bounding_radius = lod.radius;
    let simplification_error_in_object_space = lod.error;

    let world_scale = meshlet_world_mat_no_translation.scale().max_channel();

    let meshlet_bounding_center_in_render: Node<Vec4<f32>> =
      meshlet_world_mat_no_translation * (meshlet_bounding_center_in_local, val(1.)).into();
    let meshlet_bounding_center_in_render =
      meshlet_bounding_center_in_render.xyz() + translate_into_render_space;
    let meshlet_radius_world = meshlet_bounding_radius * world_scale;

    let simplification_error_in_world_space = world_scale * simplification_error_in_object_space;

    let distance = meshlet_bounding_center_in_render.length() - meshlet_radius_world;

    let distance = distance.max(self.camera_near);
    let projected_error =
      simplification_error_in_world_space / distance * val(0.5) * self.camera_projection.y().y();
    (projected_error * self.view_size.y()).less_than(pixel_error_threshold)
  }
}
