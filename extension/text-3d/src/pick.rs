use rendiation_geometry::Triangle;
use rendiation_geometry::*;
use rendiation_mesh_core::{AbstractMesh, AbstractMeshIntersectionExt};
use rendiation_scene_geometry_query::*;

use crate::*;

pub fn use_text_picker(
  cx: &mut impl DBHookCxLike,
  sys: &Arc<RwLock<FontSystem>>,
) -> Option<TextPicker> {
  let text = cx
    .use_shared_dual_query_view(Text3dSlugBuffer(sys.clone()))
    .use_assure_result(cx);

  cx.when_resolve_stage(|| TextPicker {
    text: text.expect_resolve_stage(),
    mat: read_global_db_component(),
    relation: read_global_db_foreign_key(),
  })
}

pub struct TextPicker {
  pub text: BoxedDynQuery<RawEntityHandle, ExternalRefPtr<SlugBuffer>>,
  pub relation: ForeignKeyReadView<SceneModelText3dPayload>,
  pub mat: ComponentReadView<Text3dLocalTransform>,
}

impl TextPicker {
  fn mesh_view(&self, idx: EntityHandle<SceneModelEntity>) -> Option<TextPickView> {
    let text = self.relation.get(idx)?;
    let buffer = self.text.access(text.raw_handle_ref())?;
    let mat = self.mat.get(text)?.clone();

    Some(TextPickView { buffer, mat })
  }
}

impl LocalModelPicker for TextPicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>> {
    let _ = self.mesh_view(idx)?;
    Some(None)
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    // already considered in local_tolerance
    _extra_screen_space_tolerance: f32,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint> {
    *self
      .mesh_view(idx)?
      .ray_intersect_nearest(local_ray, &FaceSide::Double)
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    // already considered in local_tolerance
    _extra_screen_space_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<()> {
    self
      .mesh_view(idx)?
      .ray_intersect_all(local_ray, &FaceSide::Double, results);
    Some(())
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    f: &Frustum,
    helper: Option<&FrustumIntersectionTestHelper<f32>>,
    policy: ObjectTestPolicy,
    // todo missing
    _extra_screen_space_tolerance: f32,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<bool> {
    let r = frustum_test_abstract_mesh(&self.mesh_view(idx)?, policy, |t| match policy {
      ObjectTestPolicy::Intersect => frustum_intersect_triangle(helper, f, t.a, t.b, t.c),
      ObjectTestPolicy::Contains => f.contains(&t.a) && f.contains(&t.b) && f.contains(&t.c),
    });

    Some(r)
  }
}

struct TextPickView {
  // todo, avoid this clone
  buffer: ExternalRefPtr<SlugBuffer>,
  mat: Mat4<f32>,
}

impl AbstractMesh for TextPickView {
  type Primitive = Triangle<Vec3<f32>>;
  fn primitive_count(&self) -> usize {
    self.buffer.hit_boxes.len()
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let bbox = self.buffer.hit_boxes.get(primitive_index / 2)?;
    let tri = if primitive_index % 2 == 0 {
      let a = Vec3::new(bbox.min.x, bbox.min.y, 0.);
      let b = Vec3::new(bbox.max.x, bbox.max.y, 0.);
      let c = Vec3::new(bbox.min.x, bbox.max.y, 0.);
      Triangle::new(a, b, c).apply_matrix_into(self.mat)
    } else {
      let a = Vec3::new(bbox.min.x, bbox.min.y, 0.);
      let b = Vec3::new(bbox.max.x, bbox.min.y, 0.);
      let c = Vec3::new(bbox.max.x, bbox.max.y, 0.);
      Triangle::new(a, b, c).apply_matrix_into(self.mat)
    };
    Some(tri)
  }
}
