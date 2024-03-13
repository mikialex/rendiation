use crate::*;

#[derive(Default, Clone, Copy)]
pub struct Cone {
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn get_meshlet_cone(acc: &Cone, triangle_count: u32) -> Cone {
  let mut result = *acc;

  let center_scale = if triangle_count == 0 {
    0.
  } else {
    1. / triangle_count as f32
  };

  result.position *= center_scale;

  let axis_length = result.direction.length2();
  let axis_scale = if axis_length == 0. {
    0.
  } else {
    1. / axis_length.sqrt()
  };

  result.direction *= axis_scale;

  result
}

pub fn compute_triangle_cones<V: Positioned<Position = Vec3<f32>>>(
  indices: &[u32],
  vertex: &[V],
) -> (Vec<Cone>, f32) {
  let mut mesh_area = 0.;

  let mut cones = Vec::with_capacity(indices.len() / 3);

  for [a, b, c] in indices.array_chunks::<3>() {
    let p0 = vertex[*a as usize].position();
    let p1 = vertex[*b as usize].position();
    let p2 = vertex[*c as usize].position();

    let p10 = p1 - p0;
    let p20 = p2 - p0;
    let mut normal = p10.cross(p20);
    let area = normal.normalize_self(); // we cal the double side of the triangle are so not need divide 2?

    let center = (p1 + p1 + p2) / 3.;
    cones.push(Cone {
      position: center,
      direction: normal,
    });
    mesh_area += area;
  }
  (cones, mesh_area)
}
