use rendiation_csg_sdf_expression::*;

use crate::*;

pub fn use_test_content_panel(cx: &mut ViewerCx) {
  let (cx, living_planes) = cx.use_plain_state::<Vec<EntityHandle<ClippingPlaneEntity>>>();

  if let ViewerCxStage::Gui {
    egui_ctx, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("test-content").or_insert(false);

    egui::Window::new("Test contents")
      .open(opened)
      .default_size((200., 200.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        let scene = cx.default_scene.scene;

        if ui.button("test csg clipping1").clicked() {
          test_csg_clipping_data1(scene)
        }

        if ui.button("test csg clipping2").clicked() {
          test_csg_clipping_data2(scene)
        }

        if ui.button("test csg clipping3").clicked() {
          test_csg_clipping_data3(scene)
        }

        if ui.button("test array plane clipping1").clicked() {
          let planes = test_array_plane_clipping_data1(scene);
          living_planes.extend(planes);
        }
        if ui.button("clear array plane clipping").clicked() {
          let mut w = global_entity_of::<ClippingPlaneEntity>().entity_writer();
          for p in living_planes.drain(..) {
            w.delete_entity(p);
          }
        }
      });
  }
}

fn test_array_plane_clipping_data1(
  scene: EntityHandle<SceneEntity>,
) -> Vec<EntityHandle<ClippingPlaneEntity>> {
  let mut w = global_entity_of::<ClippingPlaneEntity>().entity_writer();

  fn write_plane(
    w: &mut TableWriter<ClippingPlaneEntity>,
    dir: Vec3<f32>,
    constant: f32,
    scene: EntityHandle<SceneEntity>,
  ) -> EntityHandle<ClippingPlaneEntity> {
    let dir = dir.normalize();
    w.new_entity(|w| {
      w.write::<ClippingPlaneInfo>(&Vec4::new(dir.x, dir.y, dir.z, constant))
        .write::<ClippingPlaneRefScene>(&scene.some_handle())
    })
  }

  [
    write_plane(&mut w, Vec3::new(1., 0., 0.), 0., scene),
    write_plane(&mut w, Vec3::new(0., 0., 1.), 0., scene),
    write_plane(&mut w, Vec3::new(0., 1., 0.), 0., scene),
  ]
  .to_vec()
}

fn test_csg_clipping_data1(scene: EntityHandle<SceneEntity>) {
  let mut w = global_entity_of::<CSGExpressionNodeEntity>().entity_writer();

  let p1 = write_plane(&mut w, Vec3::new(1., 0., 0.), 0.);
  let p2 = write_plane(&mut w, Vec3::new(0., 0., 1.), 0.);
  let p3 = write_plane(&mut w, Vec3::new(0., 1., 0.), 0.);

  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Min))
      .write::<CSGExpressionLeftChild>(&p1.some_handle())
      .write::<CSGExpressionRightChild>(&p2.some_handle())
  });
  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Min))
      .write::<CSGExpressionLeftChild>(&root.some_handle())
      .write::<CSGExpressionRightChild>(&p3.some_handle())
  });

  global_entity_component_of::<SceneCSGClipping, _>(|c| c.write().write(scene, root.some_handle()));
}

fn write_plane(
  w: &mut TableWriter<CSGExpressionNodeEntity>,
  dir: Vec3<f32>,
  constant: f32,
) -> EntityHandle<CSGExpressionNodeEntity> {
  let plane = Plane::new(dir.into_normalized(), constant);
  let plane = CSGExpressionNode::Plane(plane);
  w.new_entity(|w| w.write::<CSGExpressionNodeContent>(&Some(plane)))
}

fn write_sphere(
  w: &mut TableWriter<CSGExpressionNodeEntity>,
  center: Vec3<f32>,
  radius: f32,
) -> EntityHandle<CSGExpressionNodeEntity> {
  let sphere = CSGExpressionNode::Sphere(Sphere::new(center, radius));
  w.new_entity(|w| w.write::<CSGExpressionNodeContent>(&Some(sphere)))
}

fn test_csg_clipping_data2(scene: EntityHandle<SceneEntity>) {
  let mut w = global_entity_of::<CSGExpressionNodeEntity>().entity_writer();

  let p1 = write_plane(&mut w, Vec3::new(-1., 0., 0.), 0.);
  let p2 = write_plane(&mut w, Vec3::new(0., 0., -1.), 0.);
  let p3 = write_plane(&mut w, Vec3::new(0., -1., 0.), 0.);

  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Max))
      .write::<CSGExpressionLeftChild>(&p1.some_handle())
      .write::<CSGExpressionRightChild>(&p2.some_handle())
  });
  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Max))
      .write::<CSGExpressionLeftChild>(&root.some_handle())
      .write::<CSGExpressionRightChild>(&p3.some_handle())
  });

  global_entity_component_of::<SceneCSGClipping, _>(|c| c.write().write(scene, root.some_handle()));
}

fn test_csg_clipping_data3(scene: EntityHandle<SceneEntity>) {
  let mut w = global_entity_of::<CSGExpressionNodeEntity>().entity_writer();

  let p1 = write_plane(&mut w, Vec3::new(-1., 0., 0.), 0.);
  let p2 = write_plane(&mut w, Vec3::new(0., 0., -1.), 0.);
  let sphere = write_sphere(&mut w, Vec3::new(0., 0., 0.), 0.5);

  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Max))
      .write::<CSGExpressionLeftChild>(&p1.some_handle())
      .write::<CSGExpressionRightChild>(&p2.some_handle())
  });
  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Min))
      .write::<CSGExpressionLeftChild>(&root.some_handle())
      .write::<CSGExpressionRightChild>(&sphere.some_handle())
  });

  global_entity_component_of::<SceneCSGClipping, _>(|c| c.write().write(scene, root.some_handle()));
}
