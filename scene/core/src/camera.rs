use crate::*;

declare_entity!(SceneCameraEntity);
declare_foreign_key!(SceneCameraBelongsToScene, SceneCameraEntity, SceneEntity);
declare_foreign_key!(SceneCameraNode, SceneCameraEntity, SceneNodeEntity);

declare_component!(
  SceneCameraPerspective,
  SceneCameraEntity,
  Option<PerspectiveProjection<f32>>
);

declare_component!(
  SceneCameraOrthographic,
  SceneCameraEntity,
  Option<OrthographicProjection<f32>>
);

pub fn register_camera_data_model() {
  global_database()
    .declare_entity::<SceneCameraEntity>()
    .declare_component::<SceneCameraPerspective>()
    .declare_component::<SceneCameraOrthographic>()
    .declare_foreign_key::<SceneCameraBelongsToScene>()
    .declare_foreign_key::<SceneCameraNode>();
}

pub fn use_camera_project_matrix(
  cx: &mut impl DBHookCxLike,
  ndc_mapper: impl NDCSpaceMapper<f32> + Copy,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Mat4<f32>>> {
  let perspective = cx
    .use_dual_query::<SceneCameraPerspective>()
    .dual_query_filter_map(move |proj| proj.map(|proj| proj.compute_projection_mat(&ndc_mapper)));

  let orth = cx
    .use_dual_query::<SceneCameraOrthographic>()
    .dual_query_filter_map(move |proj| proj.map(|proj| proj.compute_projection_mat(&ndc_mapper)));

  perspective.dual_query_select(orth)
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct CameraTransform {
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,

  pub rotation: Mat4<f64>,

  pub view: Mat4<f64>,
  pub world: Mat4<f64>,

  pub view_projection: Mat4<f64>,
  pub view_projection_inv: Mat4<f64>,
}

impl CameraTransform {
  pub fn new(proj: Mat4<f32>, world: Mat4<f64>) -> Self {
    let view = world.inverse_or_identity();
    let view_projection = proj.into_f64() * view;
    CameraTransform {
      world,
      view,
      rotation: world.extract_rotation_mat(),

      projection: proj,
      projection_inv: proj.inverse_or_identity(),
      view_projection,
      view_projection_inv: view_projection.inverse_or_identity(),
    }
  }
}

/// normalized_position: -1 to 1
pub fn cast_world_ray<T: Scalar>(
  view_projection_inv: Mat4<T>,
  normalized_position: Vec2<T>,
) -> Ray3<T> {
  let start = Vec3::new(normalized_position.x, normalized_position.y, -T::one());
  let end = Vec3::new(normalized_position.x, normalized_position.y, T::one());

  let world_start = view_projection_inv * start;
  let world_end = view_projection_inv * end;

  Ray3::from_origin_to_target(world_start, world_end)
}

pub struct GlobalCameraTransformShare<T>(pub T);

impl<T: NDCSpaceMapper + Copy + std::hash::Hash, Cx: DBHookCxLike> SharedResultProvider<Cx>
  for GlobalCameraTransformShare<T>
{
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = CameraTransform>;

  fn compute_share_key(&self) -> ShareKey {
    let mut hasher = fast_hash_collection::FastHasher::default();
    std::any::TypeId::of::<Self>().hash(&mut hasher);
    self.0.hash(&mut hasher);
    ShareKey::Hash(hasher.finish())
  }

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let projections = use_camera_project_matrix(cx, self.0);
    let node_mats = use_global_node_world_mat(cx);

    let camera_world_mat = node_mats.fanout(cx.use_db_rev_ref_tri_view::<SceneCameraNode>());

    camera_world_mat
      .dual_query_zip(projections)
      .dual_query_map(|(world, proj)| CameraTransform::new(proj, world))
  }
}
