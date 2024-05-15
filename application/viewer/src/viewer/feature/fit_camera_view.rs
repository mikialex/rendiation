fn fit_camera_view(&self) {
  //   let padding_ratio = 0.1;
  //   let scene_inner = self.scene.read();
  //   let scene = scene_inner.core.read();
  //   let camera = scene.active_camera.clone().unwrap();

  //   // get the bounding box of all selection
  //   let bbox = Box3::empty();
  //   // for model in self.selections.iter_selected() {
  //   //   let handle = model.read().attach_index().unwrap();
  //   //   let handle = scene_inner.core.read().models.get_handle(handle).unwrap();
  //   //   if let Some(b) = self.scene_bounding.get_model_bounding(handle) {
  //   //     bbox.expand_by_other(*b);
  //   //   } else {
  //   //     // for unbound model, we should include the it's coord's center point
  //   //     // todo, add a trait to support logically better center point
  //   //     let world = self.scene_derived.get_world_matrix(&model.read().node);
  //   //     bbox.expand_by_point(world.position());
  //   //   }
  //   // }

  //   if bbox.is_empty() {
  //     println!("not select any thing");
  //     return;
  //   }

  //   let camera = camera.read();

  //   let camera_world = self.scene_derived.get_world_matrix(&camera.node);
  //   let target_center = bbox.center();
  //   let mut object_radius = bbox.min.distance(target_center);

  //   // if we not even have one box
  //   if object_radius == 0. {
  //     object_radius = camera_world.position().distance(target_center);
  //   }

  //   match camera.projection {
  //     CameraProjectionEnum::Perspective(proj) => {
  //       // todo check horizon fov
  //       let half_fov = proj.fov.to_rad() / 2.;
  //       let canvas_half_size = half_fov.tan(); // todo consider near far limit
  //       let padded_canvas_half_size = canvas_half_size * (1.0 - padding_ratio);
  //       let desired_half_fov = padded_canvas_half_size.atan();
  //       let desired_distance = object_radius / desired_half_fov.sin();

  //       let look_at_dir_rev = (camera_world.position() - target_center).normalize();
  //       let desired_camera_center = look_at_dir_rev * desired_distance + target_center;
  //       // we assume camera has no parent!
  //       camera.node.set_local_matrix(Mat4::lookat(
  //         desired_camera_center,
  //         target_center,
  //         Vec3::new(0., 1., 0.),
  //       ))
  //       //
  //     }
  //     _ => {
  //       println!("only perspective camera support fit view for now")
  //     }
  //   }
}
