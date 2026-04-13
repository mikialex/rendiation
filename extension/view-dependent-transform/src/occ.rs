use crate::*;

declare_component!(SceneCameraLookAt, SceneCameraEntity, Option<Vec3<f64>>);
declare_component!(
  SceneModelViewDependentTransformOcc,
  SceneModelEntity,
  Option<OccStyleViewDepConfig>
);

pub fn register_occ_style_view_dependent_data_model() {
  global_entity_of::<SceneCameraEntity>().declare_component::<SceneCameraLookAt>();
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelViewDependentTransformOcc>();
}

/// the view_source scope decides liveness(also source scope).
pub fn use_occ_style_view_dependent_transform_data(
  cx: &mut impl DBHookCxLike,
  view_source: UseResult<BoxedDynDualQuery<ViewKey, (RawEntityHandle, Vec2<f32>)>>, /* (camera, view_size) */
  camera_transforms: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = CameraTransform>>,
) -> UseResult<BoxedDynDualQuery<ViewSceneModelKey, Mat4<f64>>> {
  let source = cx
    .use_dual_query::<SceneModelViewDependentTransformOcc>()
    .dual_query_filter_map(|v| v);

  let camera_target = cx.use_dual_query::<SceneCameraLookAt>();
  let camera_transforms = camera_transforms
    .dual_query_zip(camera_target)
    .dual_query_boxed();

  let (view_source, view_source_) = view_source.fork();

  let view_source_camera = view_source
    .dual_query_map(|v| v.0)
    .use_dual_query_hash_many_to_one(cx);

  let view_to_transform = camera_transforms
    .fanout(view_source_camera, cx)
    .dual_query_boxed();

  let view_query = view_source_
    .dual_query_map(|v| v.1)
    .dual_query_zip(view_to_transform)
    .dual_query_boxed();

  view_query
    .dual_query_cross_join(source)
    .dual_query_map(|((view_size, (transform, target)), view_dep)| {
      view_dep.view_dependent_transform(&transform, target, view_size)
    })
    .dual_query_boxed()
}

fn compute_view_dimension(camera_transform: &CameraTransform, look_at: Vec3<f64>) -> Vec3<f64> {
  let target_in_ndc = camera_transform.view_projection * look_at;
  let mat = camera_transform.view_projection_inv;
  let top = mat * Vec3::new(0., 1., target_in_ndc.z);
  let bottom = mat * Vec3::new(0., -1., target_in_ndc.z);
  let left = mat * Vec3::new(-1., 0., target_in_ndc.z);
  let right = mat * Vec3::new(1., 0., target_in_ndc.z);
  let near = mat * Vec3::new(0., 0., 0.);
  let far = mat * Vec3::new(0., 0., 1.);

  let width = right.distance_to(left);
  let height = top.distance_to(bottom);
  let depth = far.distance_to(near);

  Vec3::new(width, height, depth)
}

impl OccStyleViewDepConfig {
  pub fn view_dependent_transform(
    &self,
    camera_transform: &CameraTransform,
    camera_lookat: Option<Vec3<f64>>,
    view_size: Vec2<f32>,
  ) -> Mat4<f64> {
    if camera_lookat.is_none() {
      return Mat4::identity();
    }
    let camera_lookat = camera_lookat.unwrap();

    let mut mat = Mat4::identity();

    let dist = camera_transform.world.position().distance_to(camera_lookat);
    let view_dimension = compute_view_dimension(camera_transform, camera_lookat);
    let scale = view_dimension.y / view_size.y as f64;

    if self.mode == OccStyleMode::Triedron {
      let mut center = camera_lookat;
      let up = camera_transform.world.up();
      let eye = camera_transform.world.position();

      let f = (center - eye).normalize();
      let s = f.cross(up).normalize();
      let u = s.cross(f).normalize();

      if let OccStyleTransform::Dimension2 { offset, corner } = self.transform_ty {
        if (corner & (OccStyleCorner::Left | OccStyleCorner::Right)).bits() != 0 {
          let offset_x = offset.x as f64 * scale;
          let delta_x = s * (view_dimension.x * 0.5 - offset_x);
          if (corner & OccStyleCorner::Right).bits() != 0 {
            center += delta_x;
          } else {
            center -= delta_x;
          }
        }
        if (corner & (OccStyleCorner::Top | OccStyleCorner::Bottom)).bits() != 0 {
          let offset_y = offset.y as f64 * scale;
          let delta_y = u * (view_dimension.y * 0.5 - offset_y);
          if (corner & OccStyleCorner::Top).bits() != 0 {
            center += delta_y;
          } else {
            center -= delta_y;
          }
        }
        mat = Mat4::translate(center) * Mat4::scale(Vec3::splat(scale));
      }
    } else if self.mode == OccStyleMode::Screen2d {
      let mut center = Vec3::new(0., 0., -dist);
      if let OccStyleTransform::Dimension2 { offset, corner } = self.transform_ty {
        if (corner & (OccStyleCorner::Left | OccStyleCorner::Right)).bits() != 0 {
          center.x = -view_dimension.x * 0.5 + offset.x as f64 * scale;
          if (corner & OccStyleCorner::Right).bits() != 0 {
            center.x = -center.x
          }
        }
        if (corner & (OccStyleCorner::Top | OccStyleCorner::Bottom)).bits() != 0 {
          center.y = -view_dimension.y * 0.5 + offset.y as f64 * scale;
          if (corner & OccStyleCorner::Top).bits() != 0 {
            center.y = -center.y
          }
        }
      }

      let world_view_mat = Mat4::translate(center) * Mat4::scale(Vec3::splat(scale));
      mat = camera_transform.world * world_view_mat;
    } else if self.mode.contains(OccStyleMode::FrontCamera) {
      mat = camera_transform.world;
    } else {
      if let OccStyleTransform::Dimension3 { anchor_point } = self.transform_ty {
        let mut world_view_mat = camera_transform.view * Mat4::translate(anchor_point.into_f64());
        if self.mode.contains(OccStyleMode::NotRotate) {
          world_view_mat.a1 = 1.0;
          world_view_mat.a2 = 0.0;
          world_view_mat.a3 = 0.0;

          world_view_mat.b1 = 0.0;
          world_view_mat.b2 = 1.0;
          world_view_mat.b3 = 0.0;

          world_view_mat.c1 = 0.0;
          world_view_mat.c2 = 0.0;
          world_view_mat.c3 = 1.0;
        }
        if self.mode.contains(OccStyleMode::NotZoom) {
          world_view_mat = world_view_mat * Mat4::scale(Vec3::splat(scale));
        }
        mat = camera_transform.world * world_view_mat;
      }
    }

    mat
  }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Facet, PartialEq)]
pub struct OccStyleViewDepConfig {
  pub transform_ty: OccStyleTransform,
  #[facet(opaque)]
  pub mode: OccStyleMode,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Facet, PartialEq)]
pub enum OccStyleTransform {
  Dimension3 {
    anchor_point: Vec3<f32>,
  },
  Dimension2 {
    offset: Vec2<i32>,
    #[facet(opaque)]
    corner: OccStyleCorner,
  },
}

use bitflags::bitflags;

bitflags! {
  #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
  pub struct OccStyleMode: u32 {
    const None = 0x0000;
    const NotZoom = 0x0002;
    const NotRotate = 0x0008;
    const Triedron = 0x0020;
    const Screen2d = 0x0040;
    const FrontCamera = 0x0080;
    const NotZoomRotate = Self::NotZoom.bits() | Self::NotRotate.bits();
  }
}

bitflags! {
  #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
  pub struct OccStyleCorner: u32 {
    const Center = 0x0000;
    const Top = 0x0001;
    const Bottom = 0x0002;
    const Left = 0x0004;
    const Right = 0x0008;

    const LeftLower = Self::Bottom.bits() | Self::Left.bits();
    const LeftUpper = Self::Top.bits() | Self::Left.bits();

    const RightLower = Self::Bottom.bits() | Self::Right.bits();
    const RightUpper = Self::Top.bits() | Self::Right.bits();
  }
}
