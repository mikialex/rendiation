use crate::*;

pub fn gizmo_hook(cx: &mut UI3dCx, target: &mut Option<GizmoControlTargetState>) {
  let (cx, root) = cx.use_node_entity();
  let auto_scale = ViewAutoScalable {
    independent_scale_factor: 50.,
  };

  translation_gizmo(cx);
}

fn translation_gizmo(cx: &mut UI3dCx) {
  let mesh = todo!();

  //   let (cx, active_state) = cx.use_state::<AxisActiveState>();
  let active_state: AxisActiveState = todo!();

  let x = arrow(cx, AxisType::X, &mut active_state.x, mesh);
  let y = arrow(cx, AxisType::Y, &mut active_state.y, mesh);
  let z = arrow(cx, AxisType::Z, &mut active_state.z, mesh);
}

fn arrow(
  cx: &mut UI3dCx,
  axis: AxisType,
  axis_state: &mut ItemState,
  arrow_mesh: &AttributesMeshEntities,
) -> Option<DragStartState> {
  let (cx, node) = cx.use_node_entity(); // todo setup parent

  let (cx, material) = cx.use_unlit_material_entity(|| todo!());

  let (cx, model) =
    cx.use_scene_model_entity(|w| UIWidgetModelProxy::new(w, node, material, arrow_mesh));

  // let is_hovering

  if let Some(event) = &cx.event {
    if let Some(response) = model.event(event) {
      //
    }
  }

  None
}
