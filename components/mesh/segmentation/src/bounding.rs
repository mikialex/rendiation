use crate::*;

// static void computeBoundingSphere(float result[4], const float points[][3], size_t count)
// {
// 	assert(count > 0);

// 	// find extremum points along all 3 axes; for each axis we get a pair of points with min/max
// coordinates 	size_t pmin[3] = {0, 0, 0};
// 	size_t pmax[3] = {0, 0, 0};

// 	for (size_t i = 0; i < count; ++i)
// 	{
// 		const float* p = points[i];

// 		for (int axis = 0; axis < 3; ++axis)
// 		{
// 			pmin[axis] = (p[axis] < points[pmin[axis]][axis]) ? i : pmin[axis];
// 			pmax[axis] = (p[axis] > points[pmax[axis]][axis]) ? i : pmax[axis];
// 		}
// 	}

// 	// find the pair of points with largest distance
// 	float paxisd2 = 0;
// 	int paxis = 0;

// 	for (int axis = 0; axis < 3; ++axis)
// 	{
// 		const float* p1 = points[pmin[axis]];
// 		const float* p2 = points[pmax[axis]];

// 		float d2 = (p2[0] - p1[0]) * (p2[0] - p1[0]) + (p2[1] - p1[1]) * (p2[1] - p1[1]) + (p2[2] -
// p1[2]) * (p2[2] - p1[2]);

// 		if (d2 > paxisd2)
// 		{
// 			paxisd2 = d2;
// 			paxis = axis;
// 		}
// 	}

// 	// use the longest segment as the initial sphere diameter
// 	const float* p1 = points[pmin[paxis]];
// 	const float* p2 = points[pmax[paxis]];

// 	float center[3] = {(p1[0] + p2[0]) / 2, (p1[1] + p2[1]) / 2, (p1[2] + p2[2]) / 2};
// 	float radius = sqrtf(paxisd2) / 2;

// 	// iteratively adjust the sphere up until all points fit
// 	for (size_t i = 0; i < count; ++i)
// 	{
// 		const float* p = points[i];
// 		float d2 = (p[0] - center[0]) * (p[0] - center[0]) + (p[1] - center[1]) * (p[1] - center[1]) +
// (p[2] - center[2]) * (p[2] - center[2]);

// 		if (d2 > radius * radius)
// 		{
// 			float d = sqrtf(d2);
// 			assert(d > 0);

// 			float k = 0.5f + (radius / d) / 2;

// 			center[0] = center[0] * k + p[0] * (1 - k);
// 			center[1] = center[1] * k + p[1] * (1 - k);
// 			center[2] = center[2] * k + p[2] * (1 - k);
// 			radius = (radius + d) / 2;
// 		}
// 	}

// 	result[0] = center[0];
// 	result[1] = center[1];
// 	result[2] = center[2];
// 	result[3] = radius;
// }

#[derive(Default, Clone, Copy)]
pub struct Cone {
  pub p: Vec3<f32>,
  pub n: Vec3<f32>,
}

pub fn get_meshlet_cone(acc: &Cone, triangle_count: u32) -> Cone {
  let mut result = *acc;

  let center_scale = if triangle_count == 0 {
    0.
  } else {
    1. / triangle_count as f32
  };

  result.p *= center_scale;

  let axis_length = result.n.length2();
  let axis_scale = if axis_length == 0. {
    0.
  } else {
    1. / axis_length.sqrt()
  };

  result.n *= axis_scale;

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
      p: center,
      n: normal,
    });
    mesh_area += area;
  }
  (cones, mesh_area)
}
