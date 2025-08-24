use rendiation_mesh_core::{AttributeSemantic, AttributesMeshData};

use crate::*;

pub fn use_scene_camera_helper(cx: &mut ViewerCx) {
  let (cx, enabled) = cx.use_plain_state::<bool>();

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("camera helper").or_insert(false);

    egui::Window::new("Camera Helper")
      .open(opened)
      .default_size((100., 100.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.checkbox(enabled, "enabled");
      });
  }

  let camera_transforms =
    cx.use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.rendering.ndc));

  let main_camera = cx.viewer.scene.main_camera.into_raw();
  let helper_mesh_lines = camera_transforms.map(move |camera_transforms| {
    let mats = camera_transforms
      .iter_key_value()
      .filter_map(|(camera, transform)| {
        if camera == main_camera {
          None // skip current viewing camera
        } else {
          // we lost precision here, but for helpers it's ok(i don't care)
          Some(transform.view_projection_inv.into_f32())
        }
      });
    build_debug_lines_in_camera_space(mats)
  });

  use_immediate_helper_model(cx, helper_mesh_lines);
}

type LineBuffer = Vec<[Vec3<f32>; 2]>;
pub fn use_immediate_helper_model(cx: &mut ViewerCx, line: UseResult<LineBuffer>) {
  let line = line.use_assure_result(cx);

  let (cx, changes) = cx.use_plain_state::<Option<LineBuffer>>();

  let (cx, helper_mesh) = cx.use_state_init::<HelperLineModel>(|_| Default::default());

  match &mut cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      *changes = line.expect_resolve_stage().into();
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      if let Some(lines) = changes {
        writer.write_other_scene(cx.viewer.scene.widget_scene, |writer| {
          let lines: &[u8] = cast_slice(lines.as_slice());

          let lines = AttributesMeshData {
            attributes: vec![(AttributeSemantic::Positions, lines.to_vec())],
            indices: None,
            mode: rendiation_mesh_core::PrimitiveTopology::LineList,
            groups: Default::default(),
          };

          if let Some(model) = &mut helper_mesh.internal {
            model.replace_new_shape_and_cleanup_old(writer, lines);
          } else {
            helper_mesh.internal = UIWidgetModel::new(writer, lines).into();
          }
        })
      }
    }
    _ => {}
  }
  //
}

#[derive(Default)]
struct HelperLineModel {
  internal: Option<UIWidgetModel>,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for HelperLineModel {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    if let Some(model) = &mut self.internal {
      model.do_cleanup(cx.writer);
    }
  }
}

fn build_debug_lines_in_camera_space(
  view_projection_inv: impl Iterator<Item = Mat4<f32>>,
) -> LineBuffer {
  view_projection_inv
    .flat_map(build_debug_line_in_camera_space)
    .collect::<Vec<_>>()
}

fn build_debug_line_in_camera_space(
  view_projection_inv: Mat4<f32>,
) -> impl Iterator<Item = [Vec3<f32>; 2]> {
  let zero = 0.0001;
  let one = 0.9999;

  let near = zero;
  let far = one;
  let left = -one;
  let right = one;
  let top = one;
  let bottom = -one;

  let min = Vec3::new(near, left, bottom);
  let max = Vec3::new(far, right, top);

  line_box(min, max)
    .into_iter()
    .map(move |[a, b]| [view_projection_inv * a, view_projection_inv * b])
}

fn line_box(min: Vec3<f32>, max: Vec3<f32>) -> impl IntoIterator<Item = [Vec3<f32>; 2]> {
  let near = min.x;
  let far = max.x;
  let left = min.z;
  let right = max.z;
  let top = max.y;
  let bottom = min.y;

  let near_left_down = Vec3::new(left, bottom, near);
  let near_left_top = Vec3::new(left, top, near);
  let near_right_down = Vec3::new(right, bottom, near);
  let near_right_top = Vec3::new(right, top, near);

  let far_left_down = Vec3::new(left, bottom, far);
  let far_left_top = Vec3::new(left, top, far);
  let far_right_down = Vec3::new(right, bottom, far);
  let far_right_top = Vec3::new(right, top, far);

  [
    [near_left_down, near_left_top],
    [near_right_down, near_right_top],
    [near_left_down, near_right_down],
    [near_left_top, near_right_top],
    //
    [far_left_down, far_left_top],
    [far_right_down, far_right_top],
    [far_left_down, far_right_down],
    [far_left_top, far_right_top],
    //
    [near_left_down, far_left_down],
    [near_left_top, far_left_top],
    [near_right_down, far_right_down],
    [near_right_top, far_right_top],
  ]
}
