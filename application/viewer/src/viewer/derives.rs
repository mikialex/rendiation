use crate::*;

pub struct Viewer3dSceneDeriveSource {
  pub world_mat: RQForker<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynReactiveQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub camera_transforms: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub mesh_vertex_ref:
    RevRefOfForeignKeyWatch<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub sm_to_s: RevRefOfForeignKeyWatch<SceneModelBelongsToScene>,
  pub sm_world_bounding: BoxedDynReactiveQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub node_children:
    BoxedDynReactiveOneToManyRelation<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
}

impl Viewer3dSceneDeriveSource {
  pub fn poll_update(&self) -> Viewer3dSceneDerive {
    noop_ctx!(cx);

    let (_, world_mat) = self.world_mat.describe(cx).resolve_kept();
    let (_, node_net_visible) = self.node_net_visible.describe(cx).resolve_kept();
    let (_, camera_transforms) = self.camera_transforms.describe(cx).resolve_kept();
    let (_, mesh_vertex_ref) = self
      .mesh_vertex_ref
      .describe_with_inv_dyn(cx)
      .resolve_kept();
    let (_, sm_to_s) = self.sm_to_s.describe_with_inv_dyn(cx).resolve_kept();
    let (_, sm_world_bounding) = self.sm_world_bounding.describe(cx).resolve_kept();
    let (_, node_children) = self.node_children.describe_with_inv_dyn(cx).resolve_kept();
    Viewer3dSceneDerive {
      world_mat: world_mat.into_boxed(),
      camera_transforms: camera_transforms.into_boxed(),
      mesh_vertex_ref: mesh_vertex_ref.into_boxed_multi(),
      node_net_visible: node_net_visible.into_boxed(),
      sm_world_bounding: sm_world_bounding.into_boxed(),
      node_children: node_children.into_boxed_multi(),
      sm_to_s: sm_to_s.into_boxed_multi(),
    }
  }
}

/// used in render & scene update
#[derive(Clone)]
pub struct Viewer3dSceneDerive {
  pub world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub node_children:
    BoxedDynMultiQuery<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
  pub camera_transforms: BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub mesh_vertex_ref:
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub sm_to_s: RevRefOfForeignKey<SceneModelBelongsToScene>,
}

pub fn create_widget_cx(
  derived: &Viewer3dSceneDerive,
  scene_reader: &SceneReader,
  viewer_scene: &Viewer3dSceneCtx,
  picker: &ViewerPicker,
  canvas_resolution: Vec2<u32>,
) -> Box<dyn WidgetEnvAccess> {
  Box::new(WidgetEnvAccessImpl {
    world_mat: derived.world_mat.clone(),
    camera_node: viewer_scene.camera_node,
    camera_proj: scene_reader
      .camera
      .read::<SceneCameraPerspective>(viewer_scene.main_camera)
      .unwrap(),
    canvas_resolution,
    camera_world_ray: picker.current_mouse_ray_in_world(),
    normalized_canvas_position: picker.normalized_position_ndc(),
  }) as Box<dyn WidgetEnvAccess>
}

struct WidgetEnvAccessImpl {
  world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  camera_node: EntityHandle<SceneNodeEntity>,
  camera_proj: PerspectiveProjection<f32>,
  canvas_resolution: Vec2<u32>,
  camera_world_ray: Ray3<f64>,
  // xy -1 to 1
  normalized_canvas_position: Vec2<f32>,
}

impl WidgetEnvAccess for WidgetEnvAccessImpl {
  fn get_world_mat(&self, sm: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f64>> {
    self.world_mat.access(&sm)
  }

  fn get_camera_node(&self) -> EntityHandle<SceneNodeEntity> {
    self.camera_node
  }

  fn get_camera_perspective_proj(&self) -> PerspectiveProjection<f32> {
    self.camera_proj
  }

  fn get_camera_world_ray(&self) -> Ray3<f64> {
    self.camera_world_ray
  }

  fn get_normalized_canvas_position(&self) -> Vec2<f32> {
    self.normalized_canvas_position
  }

  fn get_view_resolution(&self) -> Vec2<u32> {
    self.canvas_resolution
  }
}
