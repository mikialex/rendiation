use crate::*;

pub fn use_gizmo(
  cx: &mut UI3dCx,
  target: &mut Option<GizmoControlTargetState>,
) -> Option<GizmoInControl> {
  let mut style = GlobalUIStyle::default();
  //   cx.dyn_cx.scoped_cx(&mut style, |cx| {});

  let (cx, root) = cx.use_node_entity();
  let auto_scale = ViewAutoScalable {
    independent_scale_factor: 50.,
  };

  //   use_translation_gizmo(cx);

  None
}

pub enum GizmoControlResult {
  Update(GizmoUpdateTargetLocal),
  StartControl,
}

fn use_translation_gizmo(
  cx: &mut UI3dCx,
  drag_start: &DragStartState,
  target: &mut GizmoControlTargetState,
) -> Option<GizmoControlResult> {
  let arrow_mesh = todo!();

  //   let (cx, active_state) = cx.use_state::<AxisActiveState>();
  let active_state: AxisActiveState = todo!();

  let x = use_arrow_model(cx, AxisType::X, &mut active_state.x);
  let y = use_arrow_model(cx, AxisType::Y, &mut active_state.y);
  let z = use_arrow_model(cx, AxisType::Z, &mut active_state.z);

  x.or(y).or(z).and_then(|res| match *res {
    TranslateDrag::StartDrag(start) => Some(GizmoControlResult::StartControl),
    TranslateDrag::Dragging(action) => {
      handle_translating(drag_start, target, &active_state, &action)
        .map(|action| GizmoControlResult::Update(GizmoUpdateTargetLocal(action)))
    }
  })
}

enum TranslateDrag {
  StartDrag(DragStartState),
  Dragging(DragTargetAction),
}

fn use_arrow_model(
  cx: &mut UI3dCx,
  axis: AxisType,
  axis_state: &mut ItemState,
) -> Option<Box<TranslateDrag>> {
  use_axis_interactive_model(cx, axis, axis_state, todo!())
}

fn use_axis_interactive_model(
  cx: &mut UI3dCx,
  axis: AxisType,
  axis_state: &mut ItemState,
  init_mesh: &AttributesMeshEntities,
) -> Option<Box<TranslateDrag>> {
  access_cx!(cx.dyn_cx, style, GlobalUIStyle);
  let color = style.get_axis_primary_color(axis);
  let color = map_color(color, *axis_state);

  let (cx, node) = cx.use_node_entity(); // todo setup parent
  let (cx, material) = cx.use_unlit_material_entity(|| todo!());
  let (cx, model) =
    cx.use_scene_model_entity(|w| UIWidgetModelProxy::new(w, node, material, init_mesh));

  // let is_hovering

  use_view_independent_node(cx, node, || axis.mat());

  if let Some(picker) = &cx.pick_testing {
    //
  }

  cx.view_update(|w| {
    w.unlit_mat_writer
      .write::<UnlitMaterialColorComponent>(*material, color.expand_with_one());
  });

  cx.event
    .as_ref()
    .and_then(|event| model.event(event))
    .map(|_| todo!())
}
