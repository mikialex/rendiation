use rendiation_algebra::*;
use rendiation_mesh_simplification::*;

fn dist_point_triangle(p: Vec3<f32>, a: Vec3<f32>, b: Vec3<f32>, c: Vec3<f32>) -> f32 {
  let ab = b - a;
  let ac = c - a;
  let ap = p - a;
  let d1 = ab.dot(ap);
  let d2 = ac.dot(ap);
  if d1 <= 0. && d2 <= 0. {
    return (p - a).length();
  }
  let bp = p - b;
  let d3 = ab.dot(bp);
  let d4 = ac.dot(bp);
  if d3 >= 0. && d4 <= d3 {
    return (p - b).length();
  }
  let vc = d1 * d4 - d3 * d2;
  if vc <= 0. && d1 >= 0. && d3 <= 0. {
    let v = d1 / (d1 - d3);
    return (p - (a + ab * v)).length();
  }
  let cp = p - c;
  let d5 = ab.dot(cp);
  let d6 = ac.dot(cp);
  if d6 >= 0. && d5 <= d6 {
    return (p - c).length();
  }
  let vb = d5 * d2 - d1 * d6;
  if vb <= 0. && d2 >= 0. && d6 <= 0. {
    let w = d2 / (d2 - d6);
    return (p - (a + ac * w)).length();
  }
  let va = d3 * d6 - d5 * d4;
  if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
    let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
    return (p - (b + (c - b) * w)).length();
  }
  let denom = 1. / (va + vb + vc);
  let v = vb * denom;
  let w = vc * denom;
  (p - (a + ab * v + ac * w)).length()
}

fn uv_sphere(slices: u32, stacks: u32) -> (Vec<Vec3<f32>>, Vec<u32>) {
  let mut positions = Vec::new();
  let mut indices = Vec::new();
  for j in 0..=stacks {
    let theta = std::f32::consts::PI * j as f32 / stacks as f32;
    for i in 0..=slices {
      let phi = 2. * std::f32::consts::PI * i as f32 / slices as f32;
      positions.push(Vec3::new(
        theta.sin() * phi.cos(),
        theta.cos(),
        theta.sin() * phi.sin(),
      ));
    }
  }
  for j in 0..stacks {
    for i in 0..slices {
      let a = j * (slices + 1) + i;
      let b = a + 1;
      let c = a + slices + 1;
      let d = c + 1;
      indices.extend_from_slice(&[a, b, c, b, d, c]);
    }
  }
  (positions, indices)
}

fn true_error_of(
  positions: &[Vec3<f32>],
  simplified_indices: &[u32],
  origin_indices: &[u32],
) -> f32 {
  let sim_tris: Vec<[Vec3<f32>; 3]> = simplified_indices
    .chunks(3)
    .map(|t| {
      [
        positions[t[0] as usize],
        positions[t[1] as usize],
        positions[t[2] as usize],
      ]
    })
    .collect();

  let mut max_err = 0.;
  for i in origin_indices.iter().copied() {
    let p = positions[i as usize];
    let d = sim_tris
      .iter()
      .map(|t| dist_point_triangle(p, t[0], t[1], t[2]))
      .fold(f32::MAX, f32::min);
    max_err = max_err.max(d);
  }
  max_err
}

#[test]
fn check_error_magnitude() {
  let (positions, indices) = uv_sphere(48, 24);
  let mut dst = vec![0u32; indices.len()];

  println!(
    "sphere: {} vertices, {} triangles, extent 2.0",
    positions.len(),
    indices.len() / 3
  );

  for target_frac in [0.5, 0.25, 0.125, 0.05, 0.01] {
    let target = (indices.len() as f32 * target_frac) as usize;
    let result = simplify_by_edge_collapse(
      &mut dst,
      &indices,
      &positions,
      None,
      EdgeCollapseConfig {
        target_index_count: target,
        target_error: f32::MAX,
        use_absolute_error: true,
        lock_border: true,
      },
    );

    let true_error = true_error_of(&positions, &dst[0..result.result_count], &indices);

    println!(
      "target {:.0}%: result_count {} ({:.0}%), reported_error {:.4}, true_error {:.4}, ratio {:.1}x",
      target_frac * 100.,
      result.result_count,
      result.result_count as f32 / indices.len() as f32 * 100.,
      result.result_error,
      true_error,
      true_error / result.result_error.max(1e-8),
    );
  }
}

#[test]
fn check_full_lod_chain() {
  let (positions, indices) = uv_sphere(48, 24);
  let mut dst = vec![0u32; indices.len()];
  let mut sloppy_dst = vec![0u32; indices.len()];
  let extent = 2.0;
  let mut prev_count = indices.len();
  let mut levels = Vec::new();
  let mut prev_error = 0.;
  let mut prev_true = 0.;

  println!("\nfull lod chain (half count + sloppy fallback):");
  loop {
    let target = prev_count / 2;
    if target < 32 * 3 {
      break;
    }

    let result = simplify_by_edge_collapse(
      &mut dst,
      &indices,
      &positions,
      None,
      EdgeCollapseConfig {
        target_index_count: target,
        target_error: f32::MAX,
        use_absolute_error: true,
        lock_border: true,
      },
    );

    let (count, error, source) = if result.result_count <= target {
      (result.result_count, result.result_error, &dst[..])
    } else {
      let fallback = simplify_sloppy(
        &mut sloppy_dst,
        &indices,
        &positions,
        None,
        target as u32,
        extent,
        true,
      );
      (
        fallback.result_count,
        fallback.result_error,
        &sloppy_dst[..],
      )
    };

    if count >= prev_count || count < 32 * 3 {
      break;
    }

    let true_error = true_error_of(&positions, &source[0..count], &indices);
    // force the error to be monotonic increasing, same as the implementation
    let error = error.max(prev_error);
    levels.push((count, error, true_error.max(prev_true)));
    prev_count = count;
    prev_error = error;
    prev_true = true_error;
  }

  // simulate the gpu level selection: iterate from coarsest to finest,
  // pick the first level whose reported projected error is under the threshold
  println!("\nsimulated gpu selection (1080p, fov 60, threshold 2px):");
  for distance in [10., 20., 50., 100., 200., 500.] {
    let ppu = 1080. * 1.732 / (2. * distance);
    let mut selected = 0;
    for i in (0..levels.len()).rev() {
      let projected = levels[i].1 * ppu;
      if projected <= 2. || i == 0 {
        selected = i;
        break;
      }
    }
    let (count, reported, true_err) = levels[selected];
    println!(
      "distance {:.0}: selected {} tris, reported {:.2}px, TRUE {:.2}px",
      distance,
      count,
      reported * ppu,
      true_err * ppu,
    );
  }
}
