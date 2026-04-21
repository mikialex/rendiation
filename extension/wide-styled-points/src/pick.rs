use rendiation_geometry::*;
use rendiation_scene_geometry_query::*;

// use rendiation_scene_geometry_query::LocalModelPicker;
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
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();

    self
      .create_view(idx)?
      .iter_pick_test(local_ray, world_mat, camera_ctx)
      .for_each(|r| {
        nearest.refresh_nearest(OptionalNearest::some(r));
      });

    *nearest
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<()> {
    self
      .create_view(idx)?
      .iter_pick_test(local_ray, world_mat, camera_ctx)
      .for_each(|r| results.push(r));

    Some(())
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    frustum: &Frustum,
    policy: ObjectTestPolicy,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<bool> {
    let mut iter = self
      .create_view(idx)?
      .iter_tri_in_local(world_mat, camera_ctx);

    let tester = |(_, tri): (usize, Triangle3D)| frustum_test_tri(frustum, &tri, policy);

    let r = match policy {
      ObjectTestPolicy::Intersect => iter.any(tester),
      ObjectTestPolicy::Contains => iter.all(tester),
    };

    Some(r)
  }
}

struct WidePointPickView<'a> {
  points: &'a [WideStyledPointVertex],
}

impl<'a> WidePointPickView<'a> {
  pub fn iter_tri_in_local(
    &self,
    world_mat: &Mat4<f64>,
    camera_ctx: &'a CameraQueryCtx,
  ) -> impl Iterator<Item = (usize, Triangle3D)> + 'a {
    // todo, support high precision
    let local_to_ndc = (camera_ctx.camera_vp * *world_mat).into_f32();
    let ndc_to_local = local_to_ndc.inverse_or_identity();

    self
      .points
      .iter()
      .enumerate()
      .flat_map(move |(primitive_index, p)| {
        let p_in_ndc = p.position.apply_matrix_into(local_to_ndc);
        p_in_ndc.xy();
        let width_half = p.width / 2.;
        let offset = Vec2::new(width_half, width_half)
          / Vec2::from(camera_ctx.camera_view_size_in_logic_pixel.into_f32());
        let max = p_in_ndc.xy() + offset;
        let min = p_in_ndc.xy() - offset;
        let z = p_in_ndc.z();

        let max = Vec3::new(max.x, max.y, z);
        let min = Vec3::new(min.x, min.y, z);
        let left_up = Vec3::new(min.x, max.y, z);
        let right_bottom = Vec3::new(max.x, min.y, z);

        let tri_a = Triangle::new(left_up, right_bottom, max);
        let tri_b = Triangle::new(left_up, min, right_bottom);

        let tri_a = tri_a.apply_matrix_into(ndc_to_local);
        let tri_b = tri_b.apply_matrix_into(ndc_to_local);

        [(primitive_index, tri_a), (primitive_index, tri_b)]
      })
  }
  pub fn iter_pick_test(
    &self,
    local_ray: Ray3<f32>,
    world_mat: &Mat4<f64>,
    camera_ctx: &'a CameraQueryCtx,
  ) -> impl Iterator<Item = MeshBufferHitPoint> + 'a {
    self
      .iter_tri_in_local(world_mat, camera_ctx)
      .filter_map(move |(primitive_index, tri)| {
        local_ray
          .intersect(&tri, &FaceSide::Double)
          .0
          .map(|hit| MeshBufferHitPoint {
            hit,
            primitive_index,
          })
      })
  }
}
