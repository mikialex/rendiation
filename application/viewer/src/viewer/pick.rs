use rendiation_gui_3d::InteractionState3d;
use rendiation_platform_event_input::PlatformEventInput;

use crate::Viewer3dSceneDerive;

struct InteractionState3dProvider {}

impl InteractionState3dProvider {
  pub fn compute_picking_state(
    dep: &Viewer3dSceneDerive,
    input: PlatformEventInput,
  ) -> InteractionState3d {
    let current_state = &input.window_state;

    InteractionState3d {
      picker: todo!(),
      mouse_world_ray: todo!(),
      intersection_group: todo!(),
      world_ray_intersected_nearest: todo!(),
    }
  }
}
