use rendiation_mesh_generator::*;

use crate::*;

pub fn use_scene_light_helper(cx: &mut ViewerCx) {
  let (cx, spot_light_enabled) = cx.use_plain_state::<bool>();
  let (cx, point_light_enabled) = cx.use_plain_state::<bool>();
  let (cx, directional_light_enabled) = cx.use_plain_state::<bool>();

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("light helper").or_insert(false);

    egui::Window::new("Light Helper")
      .open(opened)
      .default_size((100., 100.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.checkbox(spot_light_enabled, "spot light enabled");
        ui.checkbox(point_light_enabled, "point light enabled");
        ui.checkbox(directional_light_enabled, "directional light enabled");
      });
  }

  if *directional_light_enabled {
    cx.scope(|cx| {
      let world_mat = use_global_node_world_mat_view(cx);

      let helper_mesh_lines = world_mat.map_only_spawn_stage_in_thread(
        cx,
        |_| false,
        |world_mat| {
          let light_ref_node = get_db_view::<DirectionalRefNode>();

          let mut line_buffer = Vec::new();
          let mut offsets = Vec::new();

          light_ref_node.iter_key_value().for_each(|(id, node_id)| {
            let node_id = node_id.unwrap();
            offsets.push((id, line_buffer.len()));
            create_debug_line_mesh_dir_light(
              &mut line_buffer,
              world_mat.access(&node_id).unwrap().into_f32(),
            )
          });
          (line_buffer, offsets).into()
        },
      );

      let should_pick = cx.dyn_cx.message.get::<PickSceneBlocked>().is_none();
      if let Some(pick) = use_immediate_helper_model(cx, helper_mesh_lines, should_pick) {
        if let Some(pick) = pick {
          log::info!("picked dir light: {pick:?}");
          cx.viewer.content.selected_dir_light = Some(unsafe { EntityHandle::from_raw(pick) })
        } else {
          cx.viewer.content.selected_dir_light = None
        }
      }
    })
  }

  if *point_light_enabled {
    cx.scope(|cx| {
      let world_mat = use_global_node_world_mat_view(cx);

      let helper_mesh_lines = world_mat.map_only_spawn_stage_in_thread(
        cx,
        |_| false,
        |world_mat| {
          let radius = get_db_view::<PointLightCutOffDistance>();
          let light_ref_node = get_db_view::<PointLightRefNode>();

          let mut line_buffer = Vec::new();
          let mut offsets = Vec::new();

          radius.iter_key_value().for_each(|(id, radius)| {
            let node_id = light_ref_node.access(&id).unwrap().unwrap();
            offsets.push((id, line_buffer.len()));
            create_debug_line_mesh_point_light(
              &mut line_buffer,
              radius,
              world_mat.access(&node_id).unwrap().into_f32(),
            )
          });
          (line_buffer, offsets).into()
        },
      );

      let should_pick = cx.dyn_cx.message.get::<PickSceneBlocked>().is_none();
      if let Some(pick) = use_immediate_helper_model(cx, helper_mesh_lines, should_pick) {
        if let Some(pick) = pick {
          log::info!("picked point light: {pick:?}");
          cx.viewer.content.selected_point_light = Some(unsafe { EntityHandle::from_raw(pick) })
        } else {
          cx.viewer.content.selected_point_light = None
        }
      }
    })
  }

  if *spot_light_enabled {
    cx.scope(|cx| {
      let world_mat = use_global_node_world_mat_view(cx);

      let helper_mesh_lines = world_mat.map_only_spawn_stage_in_thread(
        cx,
        |_| false,
        |world_mat| {
          let half_cone_angle = get_db_view::<SpotLightHalfConeAngle>();
          let half_penumbra_angle = get_db_view::<SpotLightHalfPenumbraAngle>();
          let cutoff = get_db_view::<SpotLightCutOffDistance>();
          let light_ref_node = get_db_view::<SpotLightRefNode>();

          let mut line_buffer = Vec::new();
          let mut offsets = Vec::new();

          half_cone_angle
            .iter_key_value()
            .for_each(|(id, half_cone_angle)| {
              let node_id = light_ref_node.access(&id).unwrap().unwrap();
              offsets.push((id, line_buffer.len()));
              create_debug_line_mesh_spot_light(
                &mut line_buffer,
                half_cone_angle,
                half_penumbra_angle.access(&id).unwrap(),
                cutoff.access(&id).unwrap(),
                world_mat.access(&node_id).unwrap().into_f32(),
              )
            });
          (line_buffer, offsets).into()
        },
      );

      let should_pick = cx.dyn_cx.message.get::<PickSceneBlocked>().is_none();
      if let Some(pick) = use_immediate_helper_model(cx, helper_mesh_lines, should_pick) {
        if let Some(pick) = pick {
          log::info!("picked spot light: {pick:?}");
          cx.viewer.content.selected_spot_light = Some(unsafe { EntityHandle::from_raw(pick) })
        } else {
          cx.viewer.content.selected_spot_light = None
        }
      }
    })
  }
}

fn create_debug_line_mesh_dir_light(lines: &mut LineBuffer, world_mat: Mat4<f32>) {
  let bottom_radius = 10.;
  let arrow_body_height = 30.;
  let arrow_radius = 20.;
  let arrow_tip_height = 60.;
  let circle = create_circle(bottom_radius, 0.).transform3d_by(world_mat);

  tessellate_curve3d(lines, circle, 32);

  let arrow = [
    (bottom_radius, 0.0),
    (bottom_radius, arrow_body_height),
    (bottom_radius, arrow_body_height),
    (arrow_radius, arrow_body_height),
    (arrow_radius, arrow_body_height),
    (0., arrow_tip_height),
  ];

  let (t, r, s) = world_mat.decompose();
  let t = Mat4::translate(t);
  let s = Mat4::scale(s);
  let r = Mat4::from(r);

  let m = t * r * Mat4::rotate_x(f32::PI() / 2.) * s;
  lines.extend(
    arrow
      .into_iter()
      .map(|p| m * Vec3::new(p.0, -p.1, 0.))
      .array_chunks(),
  );
  lines.extend(
    arrow
      .into_iter()
      .map(|p| m * Vec3::new(-p.0, -p.1, 0.))
      .array_chunks(),
  );

  let m = t * r * Mat4::rotate_z(f32::PI() / 2.) * Mat4::rotate_x(f32::PI() / 2.) * s;
  lines.extend(
    arrow
      .into_iter()
      .map(|p| m * Vec3::new(p.0, -p.1, 0.))
      .array_chunks(),
  );
  lines.extend(
    arrow
      .into_iter()
      .map(|p| m * Vec3::new(-p.0, -p.1, 0.))
      .array_chunks(),
  );
}

fn create_debug_line_mesh_point_light(lines: &mut LineBuffer, radius: f32, world_mat: Mat4<f32>) {
  let (t, r, s) = world_mat.decompose();
  let t = Mat4::translate(t);
  let s = Mat4::scale(s);
  let r = Mat4::from(r);

  let line = rendiation_mesh_generator::LineSegment3D {
    start: Vec3::new(0., 0., radius),
    end: Vec3::new(0., 0., -radius),
  };

  let step = 32;
  let circle = create_circle(radius, 0.).transform3d_by(world_mat);
  tessellate_curve3d(lines, circle, step);
  tessellate_curve3d(lines, line.transform3d_by(world_mat), step);

  let mat = t * r * Mat4::rotate_x(f32::PI() / 2.) * s;
  let circle = create_circle(radius, 0.).transform3d_by(mat);
  tessellate_curve3d(lines, circle, step);
  tessellate_curve3d(lines, line.transform3d_by(mat), step);

  let mat = t * r * Mat4::rotate_y(f32::PI() / 2.) * s;
  let circle = create_circle(radius, 0.).transform3d_by(mat);
  tessellate_curve3d(lines, circle, step);
  tessellate_curve3d(lines, line.transform3d_by(mat), step);
}

fn create_debug_line_mesh_spot_light(
  lines: &mut LineBuffer,
  half_angle: f32,
  half_penumbra: f32,
  cutoff: f32,
  world_mat: Mat4<f32>,
) {
  fn build_cone(half_angle: f32, cutoff: f32, world_mat: Mat4<f32>, lines: &mut LineBuffer) {
    let radius = half_angle.tan() * cutoff;
    let angle_outlines_ends = [
      Vec3::new(-radius, 0., -cutoff),
      Vec3::new(radius, 0., -cutoff),
      Vec3::new(0., -radius, -cutoff),
      Vec3::new(0., radius, -cutoff),
    ];

    lines.extend(
      angle_outlines_ends
        .into_iter()
        .map(|ends| [world_mat.position(), world_mat * ends]),
    );

    let circle = create_circle(radius, cutoff).transform3d_by(world_mat);

    tessellate_curve3d(lines, circle, 32);
  }

  build_cone(half_angle, cutoff, world_mat, lines);
  build_cone(half_penumbra, cutoff, world_mat, lines);
}

fn create_circle(radius: f32, offset: f32) -> impl ParametricCurve3D {
  UnitCircle
    .transform_by(Mat3::scale(Vec2::splat(radius)))
    .embed_to_surface(ParametricPlane.transform3d_by(Mat4::translate((0., 0., -offset))))
}

fn tessellate_curve3d(lines: &mut LineBuffer, curve: impl ParametricCurve3D, step_count: usize) {
  let step_size = 1.0 / step_count as f32;
  for i in 0..step_count {
    let start = curve.position(step_size * i as f32);
    let end = curve.position(step_size * (i + 1) as f32);
    lines.push([start, end]);
  }
}
