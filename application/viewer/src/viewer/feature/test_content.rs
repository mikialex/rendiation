use rendiation_csg_sdf_expression::*;

use crate::*;

pub fn use_test_content_panel(cx: &mut ViewerCx) {
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
        if ui.button("load many cubes").clicked() {
          load_stress_test(&mut SceneWriter::from_global(cx.viewer.content.scene))
        }

        if ui.button("test clipping1").clicked() {
          test_clipping_data1(cx.viewer.content.scene)
        }

        if ui.button("test clipping2").clicked() {
          test_clipping_data2(cx.viewer.content.scene)
        }
        if ui.button("test clipping3").clicked() {
          test_clipping_data3(cx.viewer.content.scene)
        }
      });
  }
}

fn test_clipping_data1(scene: EntityHandle<SceneEntity>) {
  let mut w = global_entity_of::<CSGExpressionNodeEntity>().entity_writer();

  fn write_plane(
    w: &mut EntityWriter<CSGExpressionNodeEntity>,
    dir: Vec3<f32>,
    constant: f32,
  ) -> EntityHandle<CSGExpressionNodeEntity> {
    let plane = Plane::new(dir.into_normalized(), constant);
    let plane = CSGExpressionNode::Plane(plane);
    w.new_entity(|w| w.write::<CSGExpressionNodeContent>(&Some(plane)))
  }

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
  w: &mut EntityWriter<CSGExpressionNodeEntity>,
  dir: Vec3<f32>,
  constant: f32,
) -> EntityHandle<CSGExpressionNodeEntity> {
  let plane = Plane::new(dir.into_normalized(), constant);
  let plane = CSGExpressionNode::Plane(plane);
  w.new_entity(|w| w.write::<CSGExpressionNodeContent>(&Some(plane)))
}

fn write_sphere(
  w: &mut EntityWriter<CSGExpressionNodeEntity>,
  center: Vec3<f32>,
  radius: f32,
) -> EntityHandle<CSGExpressionNodeEntity> {
  let sphere = CSGExpressionNode::Sphere(Sphere::new(center, radius));
  w.new_entity(|w| w.write::<CSGExpressionNodeContent>(&Some(sphere)))
}

fn test_clipping_data2(scene: EntityHandle<SceneEntity>) {
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

fn test_clipping_data3(scene: EntityHandle<SceneEntity>) {
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
