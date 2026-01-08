use crate::*;

declare_entity!(SceneModelEntity);
declare_foreign_key!(SceneModelBelongsToScene, SceneModelEntity, SceneEntity);
declare_foreign_key!(SceneModelRefNode, SceneModelEntity, SceneNodeEntity);
declare_foreign_key!(
  SceneModelStdModelRenderPayload,
  SceneModelEntity,
  StandardModelEntity
);
pub fn register_scene_model_data_model() {
  global_database()
    .declare_entity::<SceneModelEntity>()
    .declare_foreign_key::<SceneModelBelongsToScene>()
    .declare_foreign_key::<SceneModelRefNode>()
    .declare_foreign_key::<SceneModelStdModelRenderPayload>();
}

pub struct SceneModelDataView {
  pub model: EntityHandle<StandardModelEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub node: EntityHandle<SceneNodeEntity>,
}

impl SceneModelDataView {
  pub fn write(
    &self,
    writer: &mut EntityWriter<SceneModelEntity>,
  ) -> EntityHandle<SceneModelEntity> {
    writer.new_entity(|w| {
      w.write::<SceneModelStdModelRenderPayload>(&self.model.some_handle())
        .write::<SceneModelBelongsToScene>(&self.scene.some_handle())
        .write::<SceneModelRefNode>(&self.node.some_handle())
    })
  }
}

declare_entity!(StandardModelEntity);
declare_component!(
  StandardModelRasterizationOverride,
  StandardModelEntity,
  Option<RasterizationStates>
);

use wgpu_types::*;
#[derive(Facet, Serialize, Deserialize)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RasterizationStates {
  pub depth_write_enabled: bool,
  #[facet(opaque)]
  pub depth_compare: SemanticCompareFunction,
  #[facet(opaque)]
  pub stencil: StencilState,
  #[facet(opaque)]
  pub bias: DepthBiasState,
  #[facet(opaque)]
  pub blend: Option<BlendState>,
  #[facet(opaque)]
  pub write_mask: ColorWrites,
  #[facet(opaque)]
  pub front_face: FrontFace,
  #[facet(opaque)]
  pub cull_mode: Option<Face>,
}

impl Default for RasterizationStates {
  fn default() -> Self {
    Self {
      depth_write_enabled: true,
      depth_compare: SemanticCompareFunction::Nearer,
      blend: None,
      write_mask: ColorWrites::all(),
      bias: Default::default(),
      stencil: Default::default(),
      front_face: FrontFace::Ccw,
      cull_mode: Some(Face::Back),
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[derive(Serialize, Deserialize)]
pub enum SemanticCompareFunction {
  /// Function never passes
  Never = 1,
  /// Function passes if new value nearer than existing value
  Nearer = 2,
  /// Function passes if new value is equal to existing value. When using
  /// this compare function, make sure to mark your Vertex Shader's `@builtin(position)`
  /// output as `@invariant` to prevent artifacting.
  Equal = 3,
  /// Function passes if new value is near than or equal to existing value
  NearerEqual = 4,
  /// Function passes if new value is further than existing value
  Further = 5,
  /// Function passes if new value is not equal to existing value. When using
  /// this compare function, make sure to mark your Vertex Shader's `@builtin(position)`
  /// output as `@invariant` to prevent artifacting.
  NotEqual = 6,
  /// Function passes if new value is further than or equal to existing value
  FurtherEqual = 7,
  /// Function always passes
  Always = 8,
}

declare_foreign_key!(
  StandardModelRefUnlitMaterial,
  StandardModelEntity,
  UnlitMaterialEntity
);
declare_foreign_key!(
  StandardModelRefPbrSGMaterial,
  StandardModelEntity,
  PbrSGMaterialEntity
);
declare_foreign_key!(
  StandardModelRefPbrMRMaterial,
  StandardModelEntity,
  PbrMRMaterialEntity
);
declare_foreign_key!(
  StandardModelRefAttributesMeshEntity,
  StandardModelEntity,
  AttributesMeshEntity
);
declare_foreign_key!(StandardModelRefSkin, StandardModelEntity, SceneSkinEntity);

pub fn register_std_model_data_model() {
  global_database()
    .declare_entity::<StandardModelEntity>()
    .declare_component::<StandardModelRasterizationOverride>()
    .declare_foreign_key::<StandardModelRefAttributesMeshEntity>()
    .declare_foreign_key::<StandardModelRefUnlitMaterial>()
    .declare_foreign_key::<StandardModelRefPbrSGMaterial>()
    .declare_foreign_key::<StandardModelRefPbrMRMaterial>()
    .declare_foreign_key::<StandardModelRefSkin>();
}

pub struct StandardModelDataView {
  pub material: SceneMaterialDataView,
  pub mesh: EntityHandle<AttributesMeshEntity>,
  pub skin: Option<EntityHandle<SceneSkinEntity>>,
}

impl StandardModelDataView {
  pub fn write(
    self,
    writer: &mut EntityWriter<StandardModelEntity>,
  ) -> EntityHandle<StandardModelEntity> {
    writer.new_entity(|w| {
      match self.material {
        SceneMaterialDataView::UnlitMaterial(m) => {
          w.write::<StandardModelRefUnlitMaterial>(&m.some_handle())
        }
        SceneMaterialDataView::PbrSGMaterial(m) => {
          w.write::<StandardModelRefPbrSGMaterial>(&m.some_handle())
        }
        SceneMaterialDataView::PbrMRMaterial(m) => {
          w.write::<StandardModelRefPbrMRMaterial>(&m.some_handle())
        }
      }
      .write::<StandardModelRefAttributesMeshEntity>(&self.mesh.some_handle())
      .write::<StandardModelRefSkin>(&self.skin.map(|v| v.into_raw()))
    })
  }
}

pub struct GlobalSceneModelWorldMatrix;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for GlobalSceneModelWorldMatrix {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Mat4<f64>>;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    use_global_node_world_mat(cx).fanout(cx.use_db_rev_ref_tri_view::<SceneModelRefNode>(), cx)
  }
}

pub struct SceneModelByAttributesMeshStdModelWorldBounding<T>(pub T);

impl<Cx, T> SharedResultProvider<Cx> for SceneModelByAttributesMeshStdModelWorldBounding<T>
where
  Cx: DBHookCxLike,
  T: FnOnce(&mut Cx) -> UseResult<AttributesMeshDataChangeInput> + Clone,
{
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>>;
  share_provider_hash_type_id! {SceneModelByAttributesMeshStdModelWorldBounding<()>}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let mesh_local_bounding = cx.use_shared_dual_query(AttributeMeshLocalBounding(self.0.clone()));

    let relation = cx.use_db_rev_ref_tri_view::<StandardModelRefAttributesMeshEntity>();
    let std_mesh_local_bounding = mesh_local_bounding.fanout(relation, cx);

    let scene_model_world_mat = cx.use_shared_dual_query(GlobalSceneModelWorldMatrix);

    let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
    let scene_model_local_bounding = std_mesh_local_bounding.fanout(relation, cx);

    // todo, materialize
    scene_model_world_mat
      .dual_query_intersect(scene_model_local_bounding)
      .dual_query_map(|(mat, local)| {
        let f64_box = Box3::new(local.min.into_f64(), local.max.into_f64());
        f64_box.apply_matrix_into(mat)
      })
  }
}
