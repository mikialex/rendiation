use crate::*;

pub fn load_text3d_test(s_writer: &mut SceneWriter) {
  let mut writer = global_entity_of::<Text3dEntity>().entity_writer();

  let text_3d = writer.new_entity(|w| {
    w.write::<Text3dContent>(&Some(ExternalRefPtr::new(Text3dContentInfo {
      content: String::from("Hello abcd!\nHello, World! 我是中文"),
      font_size: 12.,
      line_height: 1.2,
      scale: 0.05,
      font: Some(String::from("Cascadia Code")),
      weight: None,
      color: Vec4::new(1., 0., 0., 1.),
      italic: false,
      width: None,
      height: None,
      align: TextAlignment::Left,
    })))
  });

  let child = s_writer.create_root_child();
  s_writer.set_local_matrix(child, Mat4::translate((0., 3., 0.)));

  s_writer.model_writer.new_entity(|w| {
    w.write::<SceneModelText3dPayload>(&text_3d.some_handle())
      .write::<SceneModelBelongsToScene>(&s_writer.scene.some_handle())
      .write::<SceneModelRefNode>(&child.some_handle())
  });
}
