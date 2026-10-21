use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::test_scenes::TestRng;
use crate::test_scenes::trim::*;
use crate::trim::*;

fn encode(patches: &[TrimmedBezierPatch], accel: &dyn TrimAccel) -> TrimEncoding {
  let model = ParametricNormalizedModel {
    patches: patches.to_vec(),
    ..Default::default()
  };
  encode_trims(&model, &trim_config(), accel)
}

#[test]
fn crossing_signs() {
  let segment = |a: (f64, f64), b: (f64, f64)| {
    let (a, b) = (Vec2::new(a.0, a.1), Vec2::new(b.0, b.1));
    EncodedSegment::new(
      &TrimQuadratic {
        points: [a, (a + b) * 0.5, b],
      },
      0,
    )
  };
  let at = Vec2::new(0.5, 0.5);
  assert_eq!(crossing(&segment((1., 0.), (1., 1.)), at), 1);
  assert_eq!(crossing(&segment((1., 1.), (1., 0.)), at), -1);
  assert_eq!(crossing(&segment((0., 0.), (0., 1.)), at), 0);
  assert_eq!(crossing(&segment((0., 0.5), (1., 0.5)), at), 0);
  // Two segments sharing an end point on the ray count once.
  let (s0, s1) = (segment((1., 0.), (1., 0.5)), segment((1., 0.5), (1., 1.)));
  assert_eq!(crossing(&s0, at) + crossing(&s1, at), 1);
  // A peak touching the ray counts zero, a valley too.
  let (s0, s1) = (
    segment((1., 0.), (1.2, 0.5)),
    segment((1.2, 0.5), (1.4, 0.)),
  );
  assert_eq!(crossing(&s0, at) + crossing(&s1, at), 0);
  let (s0, s1) = (
    segment((1., 1.), (1.2, 0.5)),
    segment((1.2, 0.5), (1.4, 1.)),
  );
  assert_eq!(crossing(&s0, at) + crossing(&s1, at), 0);
}

#[test]
fn cubic_error_within_tolerance() {
  let mut rng = TestRng(11);
  for _ in 0..200 {
    let p = [(); 4].map(|_| (rng.unit(), rng.unit()));
    let tolerance = 10f64.powf(rng.range(-6., -2.));
    let c = cubic(p);
    let trim = trim_patch(vec![vec![c.clone()]]);
    let config = TrimQuadraticConfig {
      tolerance_3d: tolerance,
      min_uv_tolerance: 0.,
      max_uv_tolerance: 1.,
    };
    let q = trim_quadratics(&trim, &config);
    let n = q.segments.len();
    assert_eq!(
      n,
      cubic_piece_count(p.map(|(x, y)| Vec2::new(x, y)), tolerance)
    );
    for (i, segment) in q.segments.iter().enumerate() {
      for j in 0..=16 {
        let s = j as f64 / 16.;
        let t = (i as f64 + s) / n as f64;
        let error = (segment.eval(s) - c.curve.eval(t)).length();
        assert!(error <= tolerance * (1. + 1e-9), "{error} > {tolerance}");
      }
    }
  }
}

#[test]
fn segments_chain_bitwise() {
  for trim in trim_scenes() {
    let q = trim_quadratics(&trim, &trim_config());
    assert_eq!(q.failures, 0);
    for range in &q.loops {
      let segments: Vec<_> = q.segments[range.clone()]
        .iter()
        .map(|s| EncodedSegment::new(s, 0))
        .collect();
      for (i, s) in segments.iter().enumerate() {
        let next = &segments[(i + 1) % segments.len()];
        assert_eq!(s.p2.x.to_bits(), next.p0.x.to_bits());
        assert_eq!(s.p2.y.to_bits(), next.p0.y.to_bits());
      }
    }
  }
}

#[test]
fn classification_matches_reference() {
  let scenes = trim_scenes();
  let all = encode(&scenes, &AllSegments);
  let band = encode(&scenes, &BandGrid::DEFAULT);
  let mut rng = TestRng(7);
  for (index, trim) in scenes.iter().enumerate() {
    let reference = TrimReference::new(trim, 1e-9);
    let quadratics = trim_quadratics(trim, &trim_config());
    let mut checked = 0;
    for p in sample_points(&all, index, &mut rng, 4000) {
      let uv = clamp_uv(p);
      let uv64 = Vec2::new(uv.x as f64, uv.y as f64);
      if reference.distance(uv64) < 2. * TRIM_TOLERANCE + 1e-6 {
        continue;
      }
      checked += 1;
      let expected = reference.winding(uv64);
      let exact: i32 = quadratics
        .segments
        .iter()
        .map(|s| crossing_f64(s, uv64))
        .sum();
      assert_eq!(exact, expected, "scene {index} f64 kernel at {uv64:?}");
      assert_eq!(
        all.winding(index, p),
        expected,
        "scene {index} all at {p:?}"
      );
      assert_eq!(
        band.is_inside(index, p),
        expected != 0,
        "scene {index} band at {p:?}"
      );
    }
    assert!(checked > 3000, "scene {index}: {checked}");
  }
}

#[test]
fn band_scan_matches_all_segments_bitwise() {
  let scenes = trim_scenes();
  let band = encode(&scenes, &BandGrid::DEFAULT);
  let mut rng = TestRng(9);
  for index in 0..scenes.len() {
    let view = band.patch_view(index).unwrap();
    for p in sample_points(&band, index, &mut rng, 2000) {
      let uv = clamp_uv(p);
      let mut winding = 0;
      BandGrid::band_scan(view, uv, &mut |s| winding += crossing(s, uv));
      assert_eq!(winding, view.winding_all(uv), "scene {index} at {uv:?}");
    }
  }
}

#[test]
fn rect_and_grid_conservative() {
  let scenes = trim_scenes();
  let band = encode(&scenes, &BandGrid::DEFAULT);
  let all = encode(&scenes, &AllSegments);
  let mut rng = TestRng(13);
  let mut constant = 0;
  for index in 0..scenes.len() {
    let view = band.patch_view(index).unwrap();
    for _ in 0..2000 {
      let level = (rng.unit() * 6.) as u32;
      let n = (1u32 << level) as f32;
      let (ix, iy) = (
        (rng.unit() * n as f64) as u32,
        (rng.unit() * n as f64) as u32,
      );
      let rect = TrimRect {
        min: Vec2::new(ix as f32 / n, iy as f32 / n),
        max: Vec2::new((ix + 1) as f32 / n, (iy + 1) as f32 / n),
      };
      let class = band.classify_rect(index, &rect);
      let reference_class = all.classify_rect(index, &rect);
      if reference_class != RectClass::Partial {
        assert_eq!(class, reference_class);
      }
      if class == RectClass::Partial {
        continue;
      }
      constant += 1;
      for j in 0..=4 {
        for i in 0..=4 {
          let t = Vec2::new(i as f32 / 4., j as f32 / 4.);
          let p = rect.min + (rect.max - rect.min) * t;
          let inside = view.winding_all(clamp_uv(p)) != 0;
          assert_eq!(
            inside,
            class == RectClass::Inside,
            "scene {index} {rect:?} at {p:?}"
          );
        }
      }
    }
  }
  assert!(constant > 1000, "{constant}");
}

#[test]
fn domain_boundary_and_out_of_range() {
  let scenes = trim_scenes();
  let band = encode(&scenes, &BandGrid::DEFAULT);
  // Plate with holes: everything outside the holes is material, also beyond u = 1.
  for (u, v) in [
    (1., 0.6),
    (1.2, 0.95),
    (0.95, -0.3),
    (0.5, 1.),
    (-0.2, 0.9),
    (1., 1.),
  ] {
    assert!(band.is_inside(0, Vec2::new(u, v)), "({u}, {v})");
  }
  assert!(!band.is_inside(0, Vec2::new(0.3, 0.3)));
  // Right side on the boundary.
  for (u, v) in [(1., 0.5), (1.5, 0.5), (0.99, 0.21)] {
    assert!(band.is_inside(3, Vec2::new(u, v)), "({u}, {v})");
  }
  for (u, v) in [(1., 0.1), (0.3, 0.5), (1., 0.95)] {
    assert!(!band.is_inside(3, Vec2::new(u, v)), "({u}, {v})");
  }
}

#[test]
fn whole_domain_trim_is_downgraded() {
  let trims = [
    trim_patch(vec![polygon(
      &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)],
      EDGE,
    )]),
    trim_patch(vec![domain_boundary()]),
  ];
  for accel in [&AllSegments as &dyn TrimAccel, &BandGrid::DEFAULT] {
    let encoding = encode(&trims, accel);
    assert_eq!(encoding.header(0).kind, TRIM_KIND_INSIDE);
    assert_eq!(encoding.header(1).kind, TRIM_KIND_INSIDE);
    assert_eq!(encoding.stats.trimmed, 1);
    assert_eq!(encoding.stats.downgraded_inside, 1);
    assert!(encoding.segments.is_empty());
  }
}

fn percentile<T: Copy + Ord>(values: &[T], p: f64) -> T {
  let mut sorted = values.to_vec();
  sorted.sort();
  sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

/// Statistics and validation on a real model, run with
/// `RENDIATION_TEST_PNM=path cargo test --release -p parametric-tessellation steam -- --ignored
/// --nocapture` (default path: `temp/step-test/steam-controller.pnm` of the repository).
#[test]
#[ignore]
fn steam_statistics() {
  let path = std::env::var("RENDIATION_TEST_PNM").unwrap_or_else(|_| {
    concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../../../temp/step-test/steam-controller.pnm"
    )
    .into()
  });
  let Ok(model) = ParametricNormalizedModel::load_pnm(&path) else {
    println!("skipped, no model at {path}");
    return;
  };
  let config = TrimQuadraticConfig::for_model(&model);
  let band = encode_trims(&model, &config, &BandGrid::DEFAULT);
  let all = encode_trims(&model, &config, &AllSegments);
  let s = &band.stats;
  println!(
    "model tolerance {}, patches {}, trimmed {}",
    model.tolerance, s.patches, s.trimmed
  );
  println!(
    "downgraded inside {} outside {}, approximation failures {}",
    s.downgraded_inside, s.downgraded_outside, s.approximation_failures
  );
  let mut tolerances = s.uv_tolerances.clone();
  tolerances.sort_by(f64::total_cmp);
  println!(
    "uv tolerance min {:e} p50 {:e} max {:e}",
    tolerances[0],
    tolerances[tolerances.len() / 2],
    tolerances[tolerances.len() - 1]
  );
  let counts = &s.segment_counts;
  println!(
    "segments {} (per patch p50 {} p90 {} p99 {} max {})",
    band.segments.len(),
    percentile(counts, 0.5),
    percentile(counts, 0.9),
    percentile(counts, 0.99),
    percentile(counts, 1.)
  );
  println!(
    "build: band grid {:?}, all segments {:?}; table {} KiB, segment pool {} KiB",
    s.build_time,
    all.stats.build_time,
    band.table.len() * 4 / 1024,
    band.segments.len() * 32 / 1024
  );

  let mut band_counts = [0usize; 9];
  let mut fullest = Vec::new();
  let mut cells = [0usize; 3];
  let encoded: Vec<_> = (0..model.patches.len())
    .filter(|&i| band.header(i).kind == TRIM_KIND_BAND_GRID)
    .collect();
  for &i in &encoded {
    let view = band.patch_view(i).unwrap();
    let layout = BandGridLayout::decode(view.data);
    band_counts[layout.band_log2 as usize] += 1;
    fullest.push(
      (0..1 << layout.band_log2)
        .map(|b| layout.band(view.data, b).len())
        .max()
        .unwrap(),
    );
    let n = 1 << layout.grid_log2;
    for cy in 0..n {
      for cx in 0..n {
        cells[layout.grid_cell(view.data, cx, cy) as usize] += 1;
      }
    }
  }
  println!(
    "band count histogram (1, 2, 4, 8, 16): {:?}",
    &band_counts[..5]
  );
  println!(
    "fullest band p50 {} p90 {} p99 {} max {}",
    percentile(&fullest, 0.5),
    percentile(&fullest, 0.9),
    percentile(&fullest, 0.99),
    percentile(&fullest, 1.)
  );
  let total = cells.iter().sum::<usize>() as f64;
  println!(
    "grid cells: undetermined {:.1}%, outside {:.1}%, inside {:.1}%",
    cells[0] as f64 / total * 100.,
    cells[1] as f64 / total * 100.,
    cells[2] as f64 / total * 100.
  );

  let mut rng = TestRng(17);
  let (mut points, mut checked) = (0, 0);
  let (mut scan_mismatch, mut reference_mismatch, mut rect_mismatch) = (0, 0, 0);
  for &i in &encoded {
    let view = band.patch_view(i).unwrap();
    let patch = &model.patches[i];
    let reference = TrimReference::new(patch, 1e-7);
    let exclusion = 2. * config.uv_tolerance(&patch.surface) + 2e-6;
    for p in sample_points(&band, i, &mut rng, 32) {
      let uv = clamp_uv(p);
      points += 1;
      let mut winding = 0;
      BandGrid::band_scan(view, uv, &mut |s| winding += crossing(s, uv));
      if winding != view.winding_all(uv) {
        scan_mismatch += 1;
      }
      let uv64 = Vec2::new(uv.x as f64, uv.y as f64);
      if reference.distance(uv64) < exclusion {
        continue;
      }
      checked += 1;
      if band.is_inside(i, p) != (reference.winding(uv64) != 0) {
        if reference_mismatch < 8 {
          println!(
            "mismatch patch {i} at {uv:?}: winding {} (all {}), reference {}, distance {:e}, \
             loops {}",
            band.winding(i, p),
            all.winding(i, p),
            reference.winding(uv64),
            reference.distance(uv64),
            patch.loops.len()
          );
        }
        reference_mismatch += 1;
      }
    }
    for _ in 0..16 {
      let n = (1u32 << (rng.unit() * 6.) as u32) as f32;
      let (ix, iy) = (
        (rng.unit() * n as f64) as u32,
        (rng.unit() * n as f64) as u32,
      );
      let rect = TrimRect {
        min: Vec2::new(ix as f32 / n, iy as f32 / n),
        max: Vec2::new((ix + 1) as f32 / n, (iy + 1) as f32 / n),
      };
      let class = band.classify_rect(i, &rect);
      if class == RectClass::Partial {
        continue;
      }
      for t in [
        (0., 0.),
        (1., 0.),
        (0., 1.),
        (1., 1.),
        (0.5, 0.5),
        (0.25, 0.75),
      ] {
        let p = rect.min + (rect.max - rect.min) * Vec2::new(t.0, t.1);
        if (view.winding_all(clamp_uv(p)) != 0) != (class == RectClass::Inside) {
          rect_mismatch += 1;
        }
      }
    }
  }
  println!(
    "validation: {points} points, band scan vs all mismatches {scan_mismatch}; {checked} points \
     away from curves, mismatches vs reference {reference_mismatch}; rect mismatches \
     {rect_mismatch}"
  );
  assert_eq!(scan_mismatch, 0);
  assert_eq!(reference_mismatch, 0);
  assert_eq!(rect_mismatch, 0);
}
