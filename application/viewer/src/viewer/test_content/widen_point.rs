use crate::*;

pub fn load_widen_points_test(s_writer: &mut SceneWriter) {
  let mut writer = global_entity_of::<WideStyledPointsEntity>().entity_writer();

  let mesh_buffer = build_wide_points_mesh(|builder| {
    let mut style_id = 0;
    for i in 0..4 {
      for j in 0..4 {
        builder.push(Vec3::new(i as f32, j as f32, 0.), 30., style_id);
        style_id += 1;
      }
    }
  });

  let wide_points_model = writer.new_entity(|w| {
    w.write::<WideStyledPointsColor>(&Vec4::new(0., 0., 1., 1.))
      .write::<WideStyledPointsMeshBuffer>(&mesh_buffer)
  });

  let child = s_writer.create_root_child();
  s_writer.set_local_matrix(child, Mat4::translate((10., 5., 0.)));

  let scene = s_writer.expect_target_scene().some_handle();

  s_writer.model_writer.new_entity(|w| {
    w.write::<SceneModelWideStyledPointsRenderPayload>(&wide_points_model.some_handle())
      .write::<SceneModelBelongsToScene>(&scene)
      .write::<SceneModelRefNode>(&child.some_handle())
  });
}

#[derive(Default)]
pub struct PointListBuilder {
  points: Vec<WideStyledPointVertex>,
}

impl PointListBuilder {
  pub fn push(&mut self, position: Vec3<f32>, width: f32, style_id: u32) {
    self.points.push(WideStyledPointVertex {
      position,
      width,
      style_id,
    })
  }
}

pub fn build_wide_points_mesh(f: impl FnOnce(&mut PointListBuilder)) -> ExternalRefPtr<Vec<u8>> {
  let mut builder = PointListBuilder::default();

  f(&mut builder);

  let u8s = bytemuck::cast_slice(&builder.points);
  ExternalRefPtr::new(u8s.to_vec())
}
