use rendiation_algebra::*;
use rendiation_mesh_generator::*;
use rendiation_parametric_rendering::surface::*;

use crate::*;

pub fn load_parametric_surface_test(writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
  // Build a degree-3 NURBS surface with a 6×6 control net.
  // Interior knots at 1/3 and 2/3 produce a 3×3 Bézier decomposition.
  let nurbs = {
    let u_count = 6;
    let v_count = 6;
    let degree = 3;

    let points: Vec<Vec3<f32>> = (0..v_count)
      .flat_map(|v| {
        (0..u_count).map(move |u| {
          let x = u as f32 * 0.4 - 1.0;
          let y = v as f32 * 0.4 - 1.0;
          let z =
            ((x * 1.5).sin() * (y * 1.5).cos() * 0.8) + ((x * 3.0).cos() * (y * 3.0).sin() * 0.2);
          Vec3::new(x, y, z)
        })
      })
      .collect();

    let u_knots = vec![0., 0., 0., 0., 1. / 3., 2. / 3., 1., 1., 1., 1.];
    let v_knots = vec![0., 0., 0., 0., 1. / 3., 2. / 3., 1., 1., 1., 1.];

    NurbsSurface::from_unweighted(points, u_count, v_count, degree, degree, u_knots, v_knots)
  };

  let patches: Vec<Vec<RationalBezierSurface<f32>>> = nurbs.to_bezier_patches();

  // --- Left side: Bézier decomposition — all patches at the same position ---
  // Each patch's ParametricSurface impl evaluates to its correct sub-region
  // of the NURBS surface, so they naturally form a continuous surface.
  {
    let root = writer.create_root_child();
    writer.set_local_matrix(root, Mat4::translate((2.5, 0., 1.5)).into_f64());

    let palette = [
      Vec3::new(0.9, 0.3, 0.3),
      Vec3::new(0.3, 0.9, 0.3),
      Vec3::new(0.3, 0.3, 0.9),
      Vec3::new(0.9, 0.9, 0.3),
      Vec3::new(0.9, 0.3, 0.9),
      Vec3::new(0.3, 0.9, 0.9),
      Vec3::new(0.8, 0.5, 0.2),
      Vec3::new(0.5, 0.2, 0.8),
      Vec3::new(0.2, 0.8, 0.5),
    ];

    for (vi, row) in patches.iter().enumerate() {
      for (ui, patch) in row.iter().enumerate() {
        let color = palette[(vi * row.len() + ui) % palette.len()];

        let mesh = build_attributes_mesh(|builder| {
          builder.triangulate_parametric(patch, TessellationConfig { u: 16, v: 16 }, true);
        })
        .build();
        let mesh = writer.write_solid_attribute_mesh(mesh).mesh;

        let material = PhysicalSpecularGlossinessMaterialDataView {
          albedo: color,
          ..Default::default()
        }
        .write(&mut writer.pbr_sg_mat_writer);
        let material = SceneMaterialDataView::PbrSGMaterial(material);

        // All patches at identity relative to root — they join seamlessly
        let child = writer.create_child(root);
        writer.create_scene_model(material, mesh, child, scene);
      }
    }
  }

  // --- Right side: the full NURBS surface for comparison ---
  {
    let mesh = build_attributes_mesh(|builder| {
      builder.triangulate_parametric(&nurbs, TessellationConfig { u: 48, v: 48 }, true);
    })
    .build();
    let mesh = writer.write_solid_attribute_mesh(mesh).mesh;

    let material = PhysicalSpecularGlossinessMaterialDataView {
      albedo: Vec3::new(0.7, 0.7, 0.7),
      ..Default::default()
    }
    .write(&mut writer.pbr_sg_mat_writer);
    let material = SceneMaterialDataView::PbrSGMaterial(material);

    let child = writer.create_root_child();
    writer.set_local_matrix(child, Mat4::translate((-2.5, 0., 1.5)).into_f64());
    writer.create_scene_model(material, mesh, child, scene);
  }
}
