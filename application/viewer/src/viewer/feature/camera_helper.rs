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

  if *enabled {
    cx.scope(|cx| {
      let camera_transforms =
        cx.use_shared_dual_query(GlobalCameraTransformShare(cx.viewer.rendering.ndc));

      // due to multi view support, we disabled the filter for now
      // let main_camera = cx.viewer.scene.main_camera.into_raw();
      let main_camera = None;
      let helper_mesh_lines =
        camera_transforms.map_spawn_stage_in_thread_dual_query(cx, move |camera_transforms| {
          let (view, delta) = camera_transforms.view_delta();
          delta.iter_key_value().next()?; // skip if nothing changed
          let mats = view.iter_key_value().filter_map(|(camera, transform)| {
            if let Some(main_camera) = main_camera {
              if camera == main_camera {
                None // skip current viewing camera
              } else {
                // we lost precision here, but for helpers it's ok(i don't care)
                Some((camera, transform.view_projection_inv.into_f32()))
              }
            } else {
              Some((camera, transform.view_projection_inv.into_f32()))
            }
          });
          build_debug_lines_in_camera_space(mats).into()
        });

      use_immediate_helper_model(cx, helper_mesh_lines, false);
    })
  }
}

pub type LineBuffer = Vec<[Vec3<f32>; 2]>;
pub type OffsetBuffer = Vec<(RawEntityHandle, usize)>;
pub fn use_immediate_helper_model(
  cx: &mut ViewerCx,
  line: UseResult<Option<(LineBuffer, OffsetBuffer)>>,
  pick: bool,
) -> Option<Option<RawEntityHandle>> {
  let line = line.use_assure_result(cx);

  let (cx, changes) = cx.use_plain_state::<Option<LineBuffer>>();

  let (cx, offsets) = cx.use_plain_state::<Option<OffsetBuffer>>();
  let (cx, helper_mesh) = cx.use_state_init::<HelperLineModel>(|_| Default::default());

  match &mut cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      if let Some(c) = line.expect_resolve_stage() {
        *changes = Some(c.0);
        *offsets = Some(c.1);
      }

      if pick && cx.input.state_delta.is_left_mouse_pressing() {
        if let Some(model) = &helper_mesh.internal {
          access_cx!(cx.dyn_cx, picker, ViewerSceneModelPicker);
          if let Some(ptr_cx) = &picker.pointer_ctx {
            let model = model.model();
            if let Some(pick_result) = picker.pick_model_nearest(model, ptr_cx.world_ray) {
              let offsets = offsets.as_ref().unwrap();
              let idx = match offsets.binary_search_by(|v| v.1.cmp(&pick_result.primitive_index)) {
                Ok(idx) => idx,
                Err(idx) => idx - 1,
              };
              return Some(Some(offsets[idx].0));
            } else {
              return Some(None);
            }
          }
        }
      }

      None
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      if let Some(lines) = changes.take() {
        writer.write_other_scene(cx.viewer.content.widget_scene, |writer| {
          let lines: &[u8] = cast_slice(lines.as_slice());

          let lines = AttributesMeshData {
            attributes: vec![(AttributeSemantic::Positions, lines.to_vec())],
            indices: None,
            mode: rendiation_mesh_core::PrimitiveTopology::LineList,
          };

          if let Some(model) = &mut helper_mesh.internal {
            model.replace_new_shape_and_cleanup_old(writer, lines);
          } else {
            helper_mesh.internal = UIWidgetModel::new(writer, lines).into();
          }
        })
      }

      None
    }
    _ => None,
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
      model.do_cleanup(&mut cx.writer);
    }
  }
}

fn build_debug_lines_in_camera_space(
  view_projection_inv: impl Iterator<Item = (RawEntityHandle, Mat4<f32>)>,
) -> (LineBuffer, OffsetBuffer) {
  let mut line_buffer = Vec::new();
  let mut offsets = Vec::new();

  view_projection_inv.for_each(|(id, mat)| {
    offsets.push((id, line_buffer.len()));
    line_buffer.extend(build_debug_line_in_camera_space(mat));
  });
  (line_buffer, offsets)
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
