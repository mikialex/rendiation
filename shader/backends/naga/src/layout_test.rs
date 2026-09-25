use super::*;

/// The Std430 layout rules in rendiation_shader_api should be exactly the natural WGSL layout,
/// use naga's layouter as the reference implementation of the spec.
#[test]
fn primitive_std430_layout_matches_naga_natural_layout() {
  let sizes = [VectorSize::Bi, VectorSize::Tri, VectorSize::Quad];
  let mut primitives = vec![PrimitiveShaderValueType::f32()];
  for size in sizes {
    primitives.push(PrimitiveShaderValueType::vector(size, ScalarType::F32));
    for rows in sizes {
      primitives.push(PrimitiveShaderValueType::Matrix {
        columns: size,
        rows,
        scalar: ScalarType::F32,
      });
    }
  }

  let mut module = naga::Module::default();
  let handles: Vec<_> = primitives
    .iter()
    .map(|p| {
      let ty = naga::Type {
        name: None,
        inner: map_primitive_type(*p),
      };
      module.types.insert(ty, Span::UNDEFINED)
    })
    .collect();
  let mut layouter = naga::proc::Layouter::default();
  layouter.update(module.to_ctx()).unwrap();

  for (p, handle) in primitives.iter().zip(handles) {
    let naga_layout = layouter[handle];
    let layout = StructLayoutTarget::Std430;
    assert_eq!(p.size_of_self(layout), naga_layout.size as usize, "{p:?}");
    assert_eq!(
      p.align_of_self(layout),
      naga_layout.alignment.round_up(1) as usize,
      "{p:?}"
    );
  }
}
