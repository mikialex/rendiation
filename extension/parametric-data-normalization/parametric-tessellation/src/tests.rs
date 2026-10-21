use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::test_scenes::*;
use crate::verify::*;
use crate::*;

fn item(surface: RationalBezierSurface, depth: f64, tilt: f64) -> TessellationItem {
  let to_camera = Mat4::translate((0., 0., -depth)) * Mat4::rotate_x(tilt);
  TessellationItem {
    instance: 0,
    patch: 0,
    surface: surface.transform(to_camera),
  }
}

#[test]
fn error_within_threshold() {
  let mut rng = TestRng(3);
  let config = TessellationConfig::default();
  for case in 0..40 {
    let surface = random_surface(&mut rng, case % 2 == 1);
    let items = [item(surface, rng.range(2.5, 30.), rng.range(-1.2, 1.2))];
    let mut t = Tessellator::new(&items, test_camera(), config);
    let leaves = t.run();
    let report = measure_error(&mut t, &leaves, 6);
    assert!(report.samples > 0);
    assert!(
      report.max_pixel_error <= config.pixel_error,
      "case {case}: {report:?} {:?}",
      t.stats
    );
  }
}

#[test]
fn near_plane_crossing_terminates() {
  let mut rng = TestRng(5);
  let items = [item(random_surface(&mut rng, true), 0.5, 1.)];
  let config = TessellationConfig {
    max_level: 6,
    ..Default::default()
  };
  let mut t = Tessellator::new(&items, test_camera(), config);
  let leaves = t.run();
  assert!(!leaves.is_empty());
  assert!(t.stats.forced_leaves > 0);
  let report = check_watertight(&mut t, &leaves);
  assert_eq!(report.mismatched_vertices, 0);
}

/// A patch split into four, one part with reversed u and one transposed, so shared boundaries
/// have bitwise equal control points in different orientations.
#[test]
fn watertight_across_patches() {
  let mut rng = TestRng(9);
  for case in 0..12 {
    let parts = split_in_four(&random_surface(&mut rng, case % 2 == 0));
    let (depth, tilt) = (rng.range(3., 10.), rng.range(-1., 1.));
    let items: Vec<_> = parts.into_iter().map(|p| item(p, depth, tilt)).collect();
    let mut t = Tessellator::new(&items, test_camera(), TessellationConfig::default());
    let leaves = t.run();
    let report = check_watertight(&mut t, &leaves);
    assert_eq!(report.mismatched_vertices, 0, "case {case}: {report:?}");
    assert_eq!(report.uncovered_vertices, 0, "case {case}: {report:?}");
    assert_eq!(report.non_manifold_lines, 0, "case {case}: {report:?}");
    // The four outer sides are split in two lines each.
    assert_eq!(report.open_lines, 8, "case {case}: {report:?}");
    assert!(
      report.max_t_junction_relative < 1e-12,
      "case {case}: {report:?}"
    );
  }
}

/// The GPU structured rounds give the same bits as the cached recursion.
#[test]
fn positions_by_rounds_match() {
  let model = random_model(21, 9, (3., 12.));
  let (items, _) = camera_space_items(&model, &test_camera(), |_| true);
  let mut t = Tessellator::new(&items, test_camera(), TessellationConfig::default());
  let leaves = t.run();
  let (meshes, rounds) = t.leaf_meshes_by_rounds(&leaves);
  assert!(rounds.len() >= 2, "{rounds:?}");
  assert_eq!(
    rounds.len(),
    t.stats.position_chains.len().max(rounds.len())
  );
  for (leaf, mesh) in leaves.iter().zip(&meshes) {
    let reference = t.leaf_mesh(leaf);
    assert_eq!(reference.params, mesh.params);
    for (a, b) in reference.positions.iter().zip(&mesh.positions) {
      assert_eq!(
        [a.x, a.y, a.z].map(f64::to_bits),
        [b.x, b.y, b.z].map(f64::to_bits)
      );
    }
  }
  let report = check_mesh_watertight(&mut t, &leaves, &meshes);
  assert_eq!(report.mismatched_vertices, 0, "{report:?}");
  assert!(report.shared_lines > 0, "{report:?}");
}

/// Rounding the camera space control points to f32 changes the leaf set only where a decision
/// has a small margin.
#[test]
fn rounding_changes_only_small_margin_decisions() {
  let config = TessellationConfig::default();
  let mut total_mismatched = 0;
  for seed in 0..6 {
    let model = random_model(30 + seed, 12, (2.5, 20.));
    let (items, _) = camera_space_items(&model, &test_camera(), |_| true);
    let rounded: Vec<_> = items
      .iter()
      .map(|item| {
        let mut item = item.clone();
        for p in &mut item.surface.control_points {
          *p = Vec4::new(p.x as f32, p.y as f32, p.z as f32, p.w as f32).into_f64();
        }
        item
      })
      .collect();
    let mut t = Tessellator::new(&items, test_camera(), config);
    let leaves = t.run();
    let mut r = Tessellator::new(&rounded, test_camera(), config);
    let rounded_leaves = r.run();
    let report = compare_leaf_sets(&mut t, &leaves, &rounded_leaves);
    assert!(report.max_mismatch_margin < 1e-4, "seed {seed}: {report:?}");
    assert!(report.matched > 0);
    total_mismatched += report.mismatched;
    for leaf in &leaves {
      let margin = t.decision_margin(leaf.key);
      assert!(margin > 0. && margin.is_finite(), "{margin}");
    }
  }
  println!("mismatched leaves after f32 rounding: {total_mismatched}");
}

/// Shared face boundaries stay bitwise equal with the face origin relative control points.
#[test]
fn face_origin_keeps_shared_boundaries() {
  let mut builder = TestModelBuilder::default();
  let mut rng = TestRng(17);
  let parts = split_in_four(&random_surface(&mut rng, true));
  let shift = placement(6., 0.4) * Mat4::translate((1234.5, -987.25, 31.));
  let face = builder.face(
    parts
      .iter()
      .map(|p| p.transform(Mat4::translate((-1234.5, 987.25, -31.)))),
  );
  builder.instance(face, shift, false);
  let model = builder.model;
  let origins = face_origins(&model);
  assert!((origins[0] - Vec3::new(-1234.5, 987.25, -31.)).length() < 3.);
  let (items, _) = camera_space_items(&model, &test_camera(), |_| true);
  let mut t = Tessellator::new(&items, test_camera(), TessellationConfig::default());
  let leaves = t.run();
  let report = check_watertight(&mut t, &leaves);
  assert_eq!(report.mismatched_vertices, 0, "{report:?}");
  assert_eq!(report.open_lines, 8, "{report:?}");
}

/// Leaf classification by the trim loops: the leaves kept without fragment trimming are inside,
/// everything removed is outside, and the decisions of the kept keys do not change.
#[test]
fn trim_classification_keeps_the_material() {
  use fast_hash_collection::FastHashMap;

  use crate::test_scenes::trim::*;
  use crate::trim::*;

  let model = trimmed_random_model(31, 14, (4., 20.));
  let camera = test_camera();
  let encoding = encode_trims(&model, &trim_config(), &BandGrid::DEFAULT);
  let items = camera_space_items(&model, &camera, |_| true).0;
  let config = TessellationConfig::default();
  let plain = Tessellator::new(&items, camera, config).run();
  let mut t = Tessellator::new(&items, camera, config).with_trims(&encoding);
  let classified = t.run();
  let s = &t.stats;
  assert!(s.trim_culled > 0 && s.trim_partial_leaves > 0 && s.trim_inside_leaves > 0);

  let rect_points = |key: SubPatchKey| {
    let (us, vs) = (key.u_segment(), key.v_segment());
    let (u0, u1) = (param_to_f64(us.start()), param_to_f64(us.end()));
    let (v0, v1) = (param_to_f64(vs.start()), param_to_f64(vs.end()));
    (0..=6).flat_map(move |j| {
      (0..=6).map(move |i| {
        let (s, r) = (i as f64 / 6., j as f64 / 6.);
        Vec2::new((u0 + (u1 - u0) * s) as f32, (v0 + (v1 - v0) * r) as f32)
      })
    })
  };
  let kept: FastHashMap<_, _> = classified.iter().map(|l| (l.key, *l)).collect();
  for leaf in &classified {
    let patch = items[leaf.key.item as usize].patch as usize;
    if !leaf.trimmed {
      for p in rect_points(leaf.key) {
        assert!(encoding.is_inside(patch, p), "{leaf:?} at {p:?}");
      }
    }
  }
  let mut removed = 0;
  for leaf in &plain {
    let patch = items[leaf.key.item as usize].patch as usize;
    match kept.get(&leaf.key) {
      Some(other) => assert_eq!(other.k, leaf.k),
      None => {
        removed += 1;
        for p in rect_points(leaf.key) {
          assert!(!encoding.is_inside(patch, p), "{leaf:?} at {p:?}");
        }
      }
    }
  }
  println!(
    "{} leaves without trim classification, {} with ({removed} removed), trim culled {}, \
     inside {}, partial {}",
    plain.len(),
    classified.len(),
    s.trim_culled,
    s.trim_inside_leaves,
    s.trim_partial_leaves
  );
  assert!(removed > 0 && classified.len() + removed == plain.len());
}
