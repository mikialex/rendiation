use rendiation_shader_api::*;

use crate::harness::*;

/// the vector component references, the matrix column references and the swizzle assignment, on
/// both the native pointer and the u32 heap pointer
#[test]
fn component_references() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);

    check_vector_reference(make_local_var::<Vec4<f32>>(), u, f);
    check_matrix_reference(make_local_var::<Mat3<f32>>(), u, f);

    let heap = fake_storage_buffer::<[u32]>(0);
    check_vector_reference(u32_heap_ptr::<Vec4<f32>>(heap.clone(), 0), u, f);
    check_matrix_reference(u32_heap_ptr::<Mat3<f32>>(heap, 4), u, f);
  });
}

fn check_vector_reference(v: ShaderPtrOf<Vec4<f32>>, u: Node<u32>, f: Node<f32>) {
  v.x().store(f);
  v.a().store(v.y().load());
  v.component::<2>().store(f);
  v.index(u).store(v.index(u + val(1)).load());

  // the partial and the full swizzle views
  v.yz().store(v.load().xy());
  v.bgr().store(v.xyz().load());
  v.wzyx().store(v.load());
  v.swizzle2::<3, 0>().store(v.xw().load());

  let readonly = Vec4::<f32>::create_readonly_view_from_raw_ptr(v.raw().clone());
  keep(readonly.x().load() + readonly.b().load() + readonly.index(u).load());
}

fn check_matrix_reference(m: ShaderPtrOf<Mat3<f32>>, u: Node<u32>, f: Node<f32>) {
  m.x().store(f.splat());
  m.y().z().store(f);
  m.index(u).store(m.z().load());
  m.index(u).index(u).store(f);
  m.z().xy().store(m.x().load().zy());

  let readonly = Mat3::<f32>::create_readonly_view_from_raw_ptr(m.raw().clone());
  keep(readonly.y().load() + readonly.index(u).load());
  keep(readonly.z().x().load());
}
