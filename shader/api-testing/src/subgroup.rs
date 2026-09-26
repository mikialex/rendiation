use rendiation_shader_api::*;

use crate::harness::*;

/// subgroup and quad operations
#[test]
fn subgroup_and_quad_operations() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let v3 = f.splat::<Vec3<f32>>();

    keep(f.subgroup_add() + f.subgroup_exclusive_mul() + f.subgroup_max());
    keep(v3.subgroup_inclusive_add());
    keep(u.subgroup_and() | u.subgroup_or() | u.subgroup_xor());
    keep(u.equals(0).subgroup_all().or(u.equals(0).subgroup_any()));
    keep(u.equals(0).subgroup_ballot());

    keep(f.subgroup_broadcast(3) + f.subgroup_broadcast_first());
    keep(f.subgroup_shuffle(u) + f.subgroup_shuffle_up(u) + f.subgroup_shuffle_down(u));
    keep(f.subgroup_shuffle_xor(val(1)));

    keep(f.quad_broadcast(1));
    keep(f.quad_swap_x() + f.quad_swap_y() + f.quad_swap_diagonal());
  });
}

/// the broadcast id must be a const-expression in [0, 128)
#[test]
#[should_panic(expected = "subgroup broadcast id must be in the range [0, 128)")]
fn subgroup_broadcast_id_out_of_range() {
  build_compute(|builder| {
    keep(runtime_values(builder).f.subgroup_broadcast(128));
  });
}

/// the quad broadcast id must be a const-expression in [0, 4)
#[test]
#[should_panic(expected = "quad broadcast id must be in the range [0, 4)")]
fn quad_broadcast_id_out_of_range() {
  build_compute(|builder| {
    keep(runtime_values(builder).f.quad_broadcast(4));
  });
}
