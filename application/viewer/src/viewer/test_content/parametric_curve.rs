use rendiation_parametric_rendering::curve3d::*;

use crate::*;

/// Sample a curve into line segments.
fn sample_curve_to_vertices(
  curve: &RationalBezierCurve3d<f32>,
  sample_count: usize,
  color: Vec4<f32>,
) -> Vec<WideLineVertex> {
  let mut vertices = Vec::with_capacity(sample_count - 1);
  for i in 0..sample_count - 1 {
    let t0 = i as f32 / (sample_count - 1) as f32;
    let t1 = (i + 1) as f32 / (sample_count - 1) as f32;
    vertices.push(WideLineVertex {
      start: curve.evaluate(t0),
      end: curve.evaluate(t1),
      color,
    });
  }
  vertices
}

pub fn load_parametric_curve_test(writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
  // --- Left group: Bezier decomposition of a NURBS curve with interior knots ---
  {
    let nurbs = {
      let points = vec![
        Vec3::new(-1.5, 0.0, -0.3),
        Vec3::new(-0.5, 1.5, 0.5),
        Vec3::new(0.5, 1.2, -0.4),
        Vec3::new(1.5, 0.3, 0.3),
        Vec3::new(2.5, -0.8, -0.2),
        Vec3::new(3.0, -0.3, 0.6),
      ];
      let knots = vec![0., 0., 0., 0., 1. / 3., 2. / 3., 1., 1., 1., 1.];
      NurbsCurve3d::from_unweighted(points, 3, knots)
    };

    let curves = nurbs.to_bezier_curves();

    let palette = [
      Vec4::new(0.9, 0.3, 0.3, 1.0),
      Vec4::new(0.3, 0.9, 0.3, 1.0),
      Vec4::new(0.3, 0.3, 0.9, 1.0),
    ];

    let root = writer.create_root_child();
    writer.set_local_matrix(root, Mat4::translate((-3.0, 0., 2.5)).into_f64());

    for (i, curve) in curves.iter().enumerate() {
      let color = palette[i % palette.len()];

      let vertices = sample_curve_to_vertices(curve, 64, color);
      let buffer = ExternalRefPtr::new(vertices);

      let wide_line_model = global_entity_of::<WideLineModelEntity>()
        .entity_writer()
        .new_entity(|w| {
          w.write::<WideLineWidth>(&1.5)
            .write::<WideLineMeshBuffer>(&buffer)
        });

      let child = writer.create_child(root);
      writer.model_writer.new_entity(|w| {
        w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
          .write::<SceneModelBelongsToScene>(&scene.some_handle())
          .write::<SceneModelRefNode>(&child.some_handle())
      });
    }
  }

  // Right group: full NURBS curve for comparison (single wide line)
  {
    let nurbs = {
      let points = vec![
        Vec3::new(-1.5, 0.0, -0.3),
        Vec3::new(-0.5, 1.5, 0.5),
        Vec3::new(0.5, 1.2, -0.4),
        Vec3::new(1.5, 0.3, 0.3),
        Vec3::new(2.5, -0.8, -0.2),
        Vec3::new(3.0, -0.3, 0.6),
      ];
      let knots = vec![0., 0., 0., 0., 1. / 3., 2. / 3., 1., 1., 1., 1.];
      NurbsCurve3d::from_unweighted(points, 3, knots)
    };

    let mut vertices = Vec::with_capacity(127);
    for i in 0..127 {
      let t0 = i as f32 / 127.;
      let t1 = (i + 1) as f32 / 127.;
      vertices.push(WideLineVertex {
        start: nurbs.evaluate(t0),
        end: nurbs.evaluate(t1),
        color: Vec4::new(0.8, 0.8, 0.8, 1.0),
      });
    }

    let buffer = ExternalRefPtr::new(vertices);

    let wide_line_model = global_entity_of::<WideLineModelEntity>()
      .entity_writer()
      .new_entity(|w| {
        w.write::<WideLineWidth>(&3.0)
          .write::<WideLineMeshBuffer>(&buffer)
      });

    let root = writer.create_root_child();
    writer.set_local_matrix(root, Mat4::translate((3.0, 0., 2.5)).into_f64());

    writer.model_writer.new_entity(|w| {
      w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
        .write::<SceneModelBelongsToScene>(&scene.some_handle())
        .write::<SceneModelRefNode>(&root.some_handle())
    });
  }

  // Simple degree-3 Bézier curves as control polygon + curve overlay
  {
    let curves: Vec<(Vec<Vec3<f32>>, Vec4<f32>)> = vec![
      (
        vec![
          Vec3::new(-3.0, -1.5, 0.0),
          Vec3::new(-1.0, 0.5, 0.5),
          Vec3::new(0.5, -0.5, -0.3),
          Vec3::new(2.0, 0.8, 0.2),
        ],
        Vec4::new(0.2, 0.8, 0.2, 1.0),
      ),
      (
        vec![
          Vec3::new(-2.0, -1.0, -0.2),
          Vec3::new(0.0, 1.2, 0.8),
          Vec3::new(1.0, 1.0, -0.6),
          Vec3::new(2.5, -0.5, 0.1),
        ],
        Vec4::new(0.8, 0.2, 0.8, 1.0),
      ),
    ];

    let root = writer.create_root_child();
    writer.set_local_matrix(root, Mat4::translate((0., 0., -2.5)).into_f64());

    for (i, (points, color)) in curves.iter().enumerate() {
      let bezier = RationalBezierCurve3d::from_unweighted(points.clone(), 3);

      // Control polygon
      let mut poly_vertices = Vec::new();
      for j in 0..points.len() - 1 {
        poly_vertices.push(WideLineVertex {
          start: points[j],
          end: points[j + 1],
          color: Vec4::new(0.5, 0.5, 0.5, 1.0),
        });
      }
      let poly_buffer = ExternalRefPtr::new(poly_vertices);

      let poly_model = global_entity_of::<WideLineModelEntity>()
        .entity_writer()
        .new_entity(|w| {
          w.write::<WideLineWidth>(&1.0)
            .write::<WideLineStylePattern>(&0x0F0F)
            .write::<WideLineStyleFactor>(&8.0)
            .write::<WideLineMeshBuffer>(&poly_buffer)
        });

      // Curve
      let curve_vertices = sample_curve_to_vertices(&bezier, 80, *color);
      let curve_buffer = ExternalRefPtr::new(curve_vertices);

      let curve_model = global_entity_of::<WideLineModelEntity>()
        .entity_writer()
        .new_entity(|w| {
          w.write::<WideLineWidth>(&2.5)
            .write::<WideLineMeshBuffer>(&curve_buffer)
        });

      let offset = if i == 0 { -4.0 } else { 1.0 };
      let child = writer.create_child(root);
      writer.set_local_matrix(child, Mat4::translate((offset, 0., 0.)));

      writer.model_writer.new_entity(|w| {
        w.write::<SceneModelWideLineRenderPayload>(&poly_model.some_handle())
          .write::<SceneModelBelongsToScene>(&scene.some_handle())
          .write::<SceneModelRefNode>(&child.some_handle())
      });
      writer.model_writer.new_entity(|w| {
        w.write::<SceneModelWideLineRenderPayload>(&curve_model.some_handle())
          .write::<SceneModelBelongsToScene>(&scene.some_handle())
          .write::<SceneModelRefNode>(&child.some_handle())
      });
    }
  }
}
