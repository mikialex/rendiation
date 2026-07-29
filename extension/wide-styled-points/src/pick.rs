use rendiation_geometry::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct WideStyledPointsSceneModelLocalBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for WideStyledPointsSceneModelLocalBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let local_boxes = cx
      .use_dual_query::<WideStyledPointsMeshBuffer>()
      .use_dual_query_execute_map(cx, || {
        |_, buffer| {
          let mut bbox = Box3::empty();
          let buffer: &[WideStyledPointVertex] = cast_slice(&buffer);
          for v in buffer {
            bbox.expand_by_point(v.position);
          }
          bbox
        }
      });

    let relation = cx.use_db_rev_ref_tri_view::<SceneModelWideStyledPointsRenderPayload>();
    local_boxes.fanout(relation, cx)
  }
}

pub fn use_wide_points_picker(cx: &mut impl DBHookCxLike) -> Option<WidePointsPicker> {
  let max_size = cx
    .use_dual_query::<WideStyledPointsMeshBuffer>()
    .use_dual_query_execute_map(cx, || {
      |_, buffer| {
        // here we assume the buffer is correctly aligned
        let points: &[WideStyledPointVertex] = cast_slice(buffer.as_slice());
        let mut max_size = 0.;
        for p in points {
          max_size = max_size.max(p.width);
        }
        max_size
      }
    })
    .use_assure_result(cx);

  cx.when_resolve_stage(|| WidePointsPicker {
    points: read_global_db_component(),
    relation: read_global_db_foreign_key(),
    max_size: max_size.expect_resolve_stage().view().into_boxed(),
  })
}

pub struct WidePointsPicker {
  pub points: ComponentReadView<WideStyledPointsMeshBuffer>,
  pub relation: ForeignKeyReadView<SceneModelWideStyledPointsRenderPayload>,
  pub max_size: BoxedDynQuery<RawEntityHandle, f32>,
}

impl WidePointsPicker {
  fn create_view(&self, idx: EntityHandle<SceneModelEntity>) -> Option<WidePointPickView<'_>> {
    let point = self.relation.get(idx)?;
    let points = self.points.get(point)?;

    // here we assume the buffer is correctly aligned
    let points = cast_slice(points);
    WidePointPickView { points }.into()
  }
}

impl LocalModelPicker for WidePointsPicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>> {
    let point = self.relation.get(idx)?;
    let size = self.max_size.access(&point.raw_handle_ref())?;
    Some(Some(IntersectTolerance::new(
      size,
      ToleranceType::ScreenSpace,
    )))
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    extra_screen_space_tolerance: f32,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    let view = self.create_view(idx)?;
    let mesh = view.into_tri_mesh(world_mat, camera_ctx, extra_screen_space_tolerance);

    for (tri_index, tri) in mesh.primitive_iter().enumerate() {
      if let Some(hit) = local_ray.intersect(&tri, &FaceSide::Double).0 {
        nearest.refresh_nearest(OptionalNearest::some(MeshBufferHitPoint {
          hit,
          primitive_index: tri_index / 2,
        }));
      }
    }

    *nearest
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    extra_screen_space_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<()> {
    let view = self.create_view(idx)?;
    let mesh = view.into_tri_mesh(world_mat, camera_ctx, extra_screen_space_tolerance);

    for (tri_index, tri) in mesh.primitive_iter().enumerate() {
      if let Some(hit) = local_ray.intersect(&tri, &FaceSide::Double).0 {
        results.push(MeshBufferHitPoint {
          hit,
          primitive_index: tri_index / 2,
        });
      }
    }

    Some(())
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    frustum: &Frustum,
    helper: Option<&FrustumIntersectionTestHelper<f32>>,
    policy: ObjectTestPolicy,
    extra_screen_space_tolerance: f32,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<bool> {
    let view = self.create_view(idx)?;
    let mesh = view.into_tri_mesh(world_mat, camera_ctx, extra_screen_space_tolerance);
    let r = frustum_test_abstract_mesh(&mesh, policy, |t| {
      frustum_test_tri(helper, frustum, &t, policy)
    });

    Some(r)
  }

  fn frustum_query_local_sub_primitives(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    frustum: &Frustum,
    helper: Option<&FrustumIntersectionTestHelper<f32>>,
    policy: ObjectTestPolicy,
    extra_screen_space_tolerance: f32,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
    results: &mut Vec<u32>,
  ) -> Option<()> {
    let view = self.create_view(idx)?;
    let mesh = view.into_tri_mesh(world_mat, camera_ctx, extra_screen_space_tolerance);

    frustum_test_abstract_mesh_as_quad_all(&mesh, policy, helper, frustum, |i| {
      results.push(i as u32);
    });

    Some(())
  }
}

struct WidePointPickView<'a> {
  points: &'a [WideStyledPointVertex],
}

impl<'a> WidePointPickView<'a> {
  fn into_tri_mesh(
    self,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
    extra_screen_space_tolerance: f32,
  ) -> WidePointTriMeshView<'a> {
    let local_to_ndc = (camera_ctx.camera_vp * *world_mat).into_f32();
    let ndc_to_local = local_to_ndc.inverse_or_identity();
    let view_size_inv =
      Vec2::new(1., 1.) / Vec2::from(camera_ctx.camera_view_size_in_logic_pixel.into_f32());

    WidePointTriMeshView {
      points: self.points,
      local_to_ndc,
      ndc_to_local,
      view_size_inv,
      extra_screen_space_tolerance,
    }
  }
}

struct WidePointTriMeshView<'a> {
  points: &'a [WideStyledPointVertex],
  local_to_ndc: Mat4<f32>,
  ndc_to_local: Mat4<f32>,
  view_size_inv: Vec2<f32>,
  extra_screen_space_tolerance: f32,
}

impl AbstractMesh for WidePointTriMeshView<'_> {
  type Primitive = Triangle3D;

  fn primitive_count(&self) -> usize {
    self.points.len() * 2
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let point_index = primitive_index / 2;
    let p = self.points.get(point_index)?;

    let p_in_ndc = p.position.apply_matrix_into(self.local_to_ndc);
    let real_width = p.width + self.extra_screen_space_tolerance;
    let offset = real_width * self.view_size_inv;
    let max = p_in_ndc.xy() + offset;
    let min = p_in_ndc.xy() - offset;
    let z = p_in_ndc.z();

    let max = Vec3::new(max.x, max.y, z);
    let min = Vec3::new(min.x, min.y, z);
    let left_up = Vec3::new(min.x, max.y, z);
    let right_bottom = Vec3::new(max.x, min.y, z);

    let tri = if primitive_index % 2 == 0 {
      Triangle::new(left_up, right_bottom, max)
    } else {
      Triangle::new(left_up, min, right_bottom)
    };

    Some(tri.apply_matrix_into(self.ndc_to_local))
  }
}
