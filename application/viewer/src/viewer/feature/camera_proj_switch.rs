use crate::*;

pub struct RequestSwitchCameraProjType(pub EntityHandle<SceneCameraEntity>);
pub fn use_camera_proj_switch(cx: &mut ViewerCx) {
  let (cx, raw_request) = cx.use_plain_state_default::<Option<RequestSwitchCameraProjType>>();
  if let Some(request) = cx.dyn_cx.message.take::<RequestSwitchCameraProjType>() {
    *raw_request = Some(request);
  }

  let (cx, camera_look_at) = cx.use_plain_state_default::<Vec3<f64>>();
  if let Some(CameraAction { look_at, .. }) = cx.dyn_cx.message.get::<CameraAction>() {
    *camera_look_at = *look_at;
  }

  struct CameraSwitchProjAction {
    camera: EntityHandle<SceneCameraEntity>,
    proj: CommonProjection,
  }
  let (cx, switch_proj_action) = cx.use_plain_state_default::<Option<CameraSwitchProjAction>>();
  if let ViewerCxStage::SceneContentUpdate { writer } = &mut cx.stage {
    if let Some(CameraSwitchProjAction { camera, proj }) = switch_proj_action.take() {
      match proj {
        CommonProjection::Perspective(proj) => {
          writer
            .camera_writer
            .write::<SceneCameraOrthographic>(camera, None);
          writer
            .camera_writer
            .write::<SceneCameraPerspective>(camera, Some(proj));
        }
        CommonProjection::Orth(proj) => {
          writer
            .camera_writer
            .write::<SceneCameraPerspective>(camera, None);
          writer
            .camera_writer
            .write::<SceneCameraOrthographic>(camera, Some(proj));
        }
      }
    }
  }

  struct SuitableOrthComputeRequest {
    camera: EntityHandle<SceneCameraEntity>,
    look_at: Vec3<f64>,
  }

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.rendering.ndc))
    .use_assure_result(cx);
  let (cx, request_compute_suitable_orth) =
    cx.use_plain_state_default::<Option<SuitableOrthComputeRequest>>();
  if cx.is_resolve_stage() {
    if let Some(SuitableOrthComputeRequest { camera, look_at }) =
      request_compute_suitable_orth.take()
    {
      let camera_transforms = camera_transforms.expect_resolve_stage();
      if let Some(camera_transform) = camera_transforms.access(&camera.into_raw()) {
        let target_in_ndc = camera_transform.view_projection * look_at;
        let top = Vec3::new(0., 1., target_in_ndc.z);
        let bottom = Vec3::new(0., -1., target_in_ndc.z);
        let left = Vec3::new(-1., 0., target_in_ndc.z);
        let right = Vec3::new(1., 0., target_in_ndc.z);

        let std_orth = OrthographicProjection {
          left: -1.,
          right: 1.,
          top: 1.,
          bottom: -1.,
          near: 0.,
          far: 2000.,
        };
        let std_orth_mat = std_orth.compute_projection_mat(&OpenGLxNDC); // this should use webgpu ndc, but it's ok.

        let mat = std_orth_mat * camera_transform.view * camera_transform.view_projection_inv;

        let top_in_orth_world = mat * top;
        let bottom_in_orth_world = mat * bottom;
        let left_in_orth_world = mat * left;
        let right_in_orth_world = mat * right;

        let orth = OrthographicProjection {
          left: left_in_orth_world.x as f32,
          right: right_in_orth_world.x as f32,
          top: top_in_orth_world.y as f32,
          bottom: bottom_in_orth_world.y as f32,
          near: 0.,
          far: 2000.,
        };

        *switch_proj_action = Some(CameraSwitchProjAction {
          camera,
          proj: CommonProjection::Orth(orth),
        })
      }
    }
  }

  if let ViewerCxStage::EventHandling { .. } = cx.stage {
    if let Some(RequestSwitchCameraProjType(camera)) = raw_request.take() {
      // why we not have a reader stage??
      let camera_perspective_proj = global_entity_component_of::<SceneCameraPerspective>().read();
      let camera_orth_proj = global_entity_component_of::<SceneCameraOrthographic>().read();
      if let Some(_perspective) = camera_perspective_proj.get_value(camera).flatten() {
        *request_compute_suitable_orth = Some(SuitableOrthComputeRequest {
          camera,
          look_at: *camera_look_at,
        });
      } else if let Some(_orth) = camera_orth_proj.get_value(camera).flatten() {
        *switch_proj_action = Some(CameraSwitchProjAction {
          camera,
          proj: CommonProjection::Perspective(Default::default()),
        })
      }
    }
  }
}
