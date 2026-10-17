use std::{alloc::Allocator, ops::ControlFlow};

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
  fn read_view(
    &self,
    text: EntityHandle<Text3dEntity>,
  ) -> Result<TextPickView, LocalModelPickDataError> {
    let err = LocalModelPickDataError;
    let buffer = self
      .text
      .access(text.raw_handle_ref())
      .ok_or(err("text layout buffer"))?;
    let mat = *self.mat.get(text).ok_or(err("text local transform"))?;
    Ok(TextPickView { buffer, mat })
  }
}

impl LocalModelPickerResolver for TextPicker {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>> {
    let text = self.relation.get(idx)?;
    let view = self.read_view(text);
    Some(view.map(|view| Box::new_in(view, alloc) as _))
  }
}

impl ResolvedLocalModelPicker for TextPickView {
  fn tolerance(&self) -> Option<IntersectTolerance> {
    None
  }

  fn primitive_count(&self) -> Result<u32, LocalModelPickDataError> {
    Ok(self.buffer.hit_boxes.len() as u32)
  }

  fn ray_query_local(
    &self,
    req: LocalRayQuery,
    v: &mut dyn LocalRayHitVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    // todo extra_screen_space_tolerance
    let hits = self.ray_intersect_iter(req.local_ray, &FaceSide::Double);
    Ok(visit_mesh_ray_hits(
      &req.local_ray,
      hits,
      |i| (i / 2) as u32,
      v,
    ))
  }

  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    // todo extra_screen_space_tolerance
    Ok(visit_frustum_quads(
      self.primitive_iter(),
      req.policy,
      req.helper,
      req.local_frustum,
      v,
    ))
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
    self.buffer.hit_boxes.len() * 2
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let bbox = self.buffer.hit_boxes.get(primitive_index / 2)?;
    let tri = if primitive_index.is_multiple_of(2) {
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
