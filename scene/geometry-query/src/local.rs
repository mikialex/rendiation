//! Helpers for implementing [LocalModelPicker].

use crate::*;

/// Convert mesh intersection results into local ray hits and push them into the visitor.
///
/// `primitive_map` maps the mesh primitive index into the picker defined primitive index, for
/// example `|i| (i / 2) as u32` for meshes that represent each logical primitive by a quad.
pub fn visit_mesh_ray_hits(
  local_ray: &Ray3<f32>,
  hits: impl Iterator<Item = MeshBufferHitPoint>,
  primitive_map: impl Fn(usize) -> u32,
  v: &mut dyn LocalRayHitVisitor,
) -> ControlFlow<()> {
  for hit in hits {
    let position = hit.hit.position;
    let t = (position - local_ray.origin).dot(*local_ray.direction);
    if t > v.t_max() {
      continue;
    }
    v.visit(LocalRayHit {
      primitive: primitive_map(hit.primitive_index),
      t,
      position,
    })?;
  }
  ControlFlow::Continue(())
}

/// Test primitives one by one and emit single primitive events.
pub fn visit_frustum_primitives<P>(
  primitives: impl Iterator<Item = P>,
  tester: impl Fn(&P) -> bool,
  v: &mut dyn FrustumPrimitiveVisitor,
) -> ControlFlow<()> {
  for (i, p) in primitives.enumerate() {
    v.visit(FrustumPrimitiveEvent::single(i as u32, tester(&p)))?;
  }
  ControlFlow::Continue(())
}

/// For meshes that represent each logical primitive (glyph, cell unit, point sprite) by a quad of
/// two adjacent triangles, the quad index is the primitive index.
pub fn visit_frustum_quads(
  triangles: impl Iterator<Item = Triangle3D>,
  policy: ObjectTestPolicy,
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  frustum: &Frustum,
  v: &mut dyn FrustumPrimitiveVisitor,
) -> ControlFlow<()> {
  for (i, [t1, t2]) in triangles.array_chunks::<2>().enumerate() {
    let pass = match policy {
      ObjectTestPolicy::Intersect => {
        frustum_test_tri(helper, frustum, &t1, policy)
          || frustum_test_tri(helper, frustum, &t2, policy)
      }
      ObjectTestPolicy::Contains => {
        frustum_test_tri(helper, frustum, &t1, policy)
          && frustum_test_tri(helper, frustum, &t2, policy)
      }
    };
    v.visit(FrustumPrimitiveEvent::single(i as u32, pass))?;
  }
  ControlFlow::Continue(())
}

#[inline(always)]
pub fn frustum_test_tri(
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  f: &Frustum,
  triangle: &Triangle3D,
  policy: ObjectTestPolicy,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => {
      frustum_intersect_triangle(helper, f, triangle.a, triangle.b, triangle.c)
    }
    ObjectTestPolicy::Contains => {
      f.contains(&triangle.a) && f.contains(&triangle.b) && f.contains(&triangle.c)
    }
  }
}

#[inline(always)]
pub fn frustum_test_line_segment(
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  f: &Frustum,
  line: &LineSegment3D,
  policy: ObjectTestPolicy,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => frustum_intersect_line_segment(helper, f, line.start, line.end),
    ObjectTestPolicy::Contains => f.contains(&line.start) && f.contains(&line.end),
  }
}
