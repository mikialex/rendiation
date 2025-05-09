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

  use_view_dependent_root(cx, root, auto_scale, |cx| {

    //   use_translation_gizmo(cx);
  });

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

  use_state_cx_in_mounting(cx, create_arrow_mesh, |cx| {
    let x = use_arrow_model(cx, AxisType::X);
    let y = use_arrow_model(cx, AxisType::Y);
    let z = use_arrow_model(cx, AxisType::Z);

    x.or(y).or(z).and_then(|res| match *res {
      TranslateDrag::StartDrag(start) => {
        //
        Some(GizmoControlResult::StartControl)
      }
      TranslateDrag::Dragging(action) => {
        handle_translating(drag_start, target, &active_state, &action)
          .map(|action| GizmoControlResult::Update(GizmoUpdateTargetLocal(action)))
      }
    })
  })
}

fn create_arrow_mesh(cx: &mut UI3dBuildCx) -> AttributesMeshEntities {
  cx.writer
    .write_attribute_mesh(ArrowShape::default().build().build())
}

// fn provide_arrow_mesh_init(cx: &mut UI3dCx){

// }

enum TranslateDrag {
  StartDrag(DragStartState),
  Dragging(DragTargetAction),
}

fn use_arrow_model(cx: &mut UI3dCx, axis: AxisType) -> Option<Box<TranslateDrag>> {
  use_axis_interactive_model(cx, axis)
}

fn use_axis_interactive_model(cx: &mut UI3dCx, axis: AxisType) -> Option<Box<TranslateDrag>> {
  let (cx, node) = cx.use_node_entity(); // todo setup parent
  use_view_independent_node(cx, node, move || axis.mat());

  let (cx, material) = cx.use_unlit_material_entity(|| todo!());
  let (cx, model) = cx.use_state_init(|cx| {
    access_cx!(cx.cx, mesh, AttributesMeshEntities);
    UIWidgetModelProxy::new(cx.writer, node, material, mesh)
  });

  cx.on_update(|w, dcx| {
    access_cx!(dcx, style, GlobalUIStyle);
    access_cx!(dcx, axis_state, AxisActiveState);
    access_cx!(dcx, item_state, ItemState);

    let color = style.get_axis_primary_color(axis);
    let color = map_color(color, *item_state);
    let self_active = item_state.active;
    let visible = !axis_state.has_any_active() || self_active;
    let color = map_color(color, *item_state);

    w.unlit_mat_writer
      .write::<UnlitMaterialColorComponent>(*material, color.expand_with_one());
    w.node_writer
      .write::<SceneNodeVisibleComponent>(*node, visible);
  });

  use_interactive_ui_widget_model(cx, model).map(|res| {
    access_cx_mut!(cx.dyn_cx, item_state, ItemState);
    if res.mouse_entering {
      item_state.hovering = true;
    }
    if res.mouse_leave {
      item_state.hovering = false;
    }
    res.mouse_down.map(|point| {
      item_state.active = true;

      // access_cx!(cx, target, Option::<GizmoControlTargetState>);
      // if let Some(target) = target {
      //   let drag_start_info = target.start_drag(point.position);
      //   access_cx_mut!(cx, drag_start, Option::<DragStartState>);
      //   debug_print("start drag");
      //   *drag_start = Some(drag_start_info);
      //   cx.message.put(GizmoInControl);
      // }
    });
    todo!()
  })
}
