use crate::*;

pub fn load_text3d_test(s_writer: &mut SceneWriter) {
  let mut writer = global_entity_of::<Text3dEntity>().entity_writer();

  {
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
        underline: false,
      })))
    });

    let child = s_writer.create_root_child();
    s_writer.set_local_matrix(child, Mat4::translate((0., 3., 0.)));

    let scene = s_writer.expect_target_scene().some_handle();
    s_writer.model_writer.new_entity(|w| {
      w.write::<SceneModelText3dPayload>(&text_3d.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&child.some_handle())
    });
  }

  {
    let text_3d = writer.new_entity(|w| {
      w.write::<Text3dContent>(&Some(ExternalRefPtr::new(Text3dContentInfo {
        content: String::from("jnkjnknjkj kjnkjnkjnk ddddddddddddddddd"),
        font_size: 12.,
        line_height: 1.2,
        scale: 0.05,
        font: Some(String::from("Cascadia Code")),
        weight: Some(700),
        color: Vec4::new(0., 0., 0., 1.),
        italic: true,
        width: Some(50.),
        height: None,
        align: TextAlignment::Left,
        underline: true,
      })))
    });

    let child = s_writer.create_root_child();
    s_writer.set_local_matrix(child, Mat4::translate((0., 13., 0.)));

    let scene = s_writer.expect_target_scene().some_handle();
    s_writer.model_writer.new_entity(|w| {
      w.write::<SceneModelText3dPayload>(&text_3d.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&child.some_handle())
    });
  }
}
