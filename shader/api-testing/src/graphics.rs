use rendiation_shader_api::*;

use crate::harness::*;

both!(IOFloat, f32);
both!(IOVec3F32, Vec3<f32>);
both!(IOVec4U32, Vec4<u32>);
both!(IOVec2I32, Vec2<i32>);

/// the user defined inter stage IO with numeric scalar and vector, the integer types are flat
/// interpolated
#[test]
fn user_defined_io() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      builder
        .expect_vertex_shader()
        .push_single_vertex_layout::<IOVec3F32>(VertexStepMode::Vertex);
      let position = builder.query::<IOVec3F32>();
      let index = builder.query::<VertexIndex>();

      builder.set_vertex_out::<IOFloat>(index.into_f32());
      builder.set_vertex_out::<IOVec3F32>(position);
      builder.set_vertex_out::<IOVec4U32>(index.splat::<Vec4<u32>>());
      builder.set_vertex_out_with_given_interpolate::<IOVec2I32>(
        index.into_i32().splat::<Vec2<i32>>(),
        ShaderInterpolation::Flat,
      );
    });
    builder.fragment(|builder, _| {
      let f = builder.query::<IOFloat>();
      let v = builder.query::<IOVec3F32>();
      let u = builder.query::<IOVec4U32>();
      let i = builder.query::<IOVec2I32>();
      keep(f.splat::<Vec3<f32>>() + v);
      keep(u.x() + i.x().into_u32());
    });
  });
}

/// the built-in values: clip distances, primitive index, and the fragment subgroup values
#[test]
fn builtin_values() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>().into_f32();
      let distances = make_local_var::<[f32; 2]>();
      distances.index(0).store(index);
      distances.index(1).store(-index);
      builder
        .expect_vertex_shader()
        .set_clip_distances(distances.load());
    });
    builder.fragment(|builder, _| {
      let index = builder.query::<FragmentPrimitiveIndex>();
      let size = builder.query::<FragmentSubgroupSize>();
      let id = builder.query::<FragmentSubgroupInvocationId>();
      keep(index + size + id);
    });
  });
}
