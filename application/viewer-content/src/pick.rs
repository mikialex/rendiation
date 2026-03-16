use crate::*;

pub fn use_viewer_scene_model_picker_impl<Cx: DBHookCxLike>(
  cx: &mut Cx,
) -> Option<Box<dyn SceneModelPicker>> {
  let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
  let node_net_visible = use_global_node_net_visible_view(cx).use_assure_result(cx);

  let use_attribute_mesh_picker = use_attribute_mesh_picker(cx, viewer_mesh_input);
  let wide_line_picker = use_wide_line_picker(cx);

  cx.when_resolve_stage(|| {
    let att_mesh_picker = use_attribute_mesh_picker.unwrap();
    let wide_line_picker = wide_line_picker.unwrap();

    let local_model_pickers: Vec<Box<dyn LocalModelPicker>> =
      vec![Box::new(att_mesh_picker), Box::new(wide_line_picker)];

    let scene_model_picker = SceneModelPickerBaseImpl {
      internal: local_model_pickers,
      scene_model_node: read_global_db_foreign_key(),
      node_world: node_world
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      node_net_visible: node_net_visible
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      filter: Some(Box::new(create_clip_pick_filter())),
    };

    Box::new(scene_model_picker) as Box<dyn SceneModelPicker>
  })
}
