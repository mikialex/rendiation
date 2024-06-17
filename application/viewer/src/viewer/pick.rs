use rendiation_gui_3d::*;

use crate::Viewer3dSceneDerive;

struct InteractionState3dProvider {}

impl InteractionState3dProvider {
  pub fn compute_picking_state(
    dep: &Viewer3dSceneDerive,
    input: PlatformEventInput,
  ) -> InteractionState3d {
    let mouse_position = &input.window_state.mouse_position;
    let window_size = &input.window_state.size;

    let normalized_position =
      compute_normalized_position_in_canvas_coordinate(*mouse_position, *window_size);

    InteractionState3d {
      picker: todo!(),
      mouse_world_ray: todo!(),
      intersection_group: todo!(),
      world_ray_intersected_nearest: todo!(),
    }
  }
}

pub fn compute_normalized_position_in_canvas_coordinate(
  offset: (f32, f32),
  size: (f32, f32),
) -> (f32, f32) {
  (offset.0 / size.0 * 2. - 1., -(offset.1 / size.1 * 2. - 1.))
}
