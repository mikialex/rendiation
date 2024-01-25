use crate::*;

#[derive(Clone)]
pub struct SceneGPUResource {
  // exist if scene is env background
  cube_env: Option<GPUCubeTextureView>,
  nodes: Arc<dyn ReactiveCollection<NodeIdentity, NodeGPU>>,
}

impl std::fmt::Debug for SceneGPUResource {
  fn fmt(&self, f: &mut __core::fmt::Formatter<'_>) -> __core::fmt::Result {
    f.debug_struct("SceneGPUResource").finish()
  }
}
impl PartialEq for SceneGPUResource {
  fn eq(&self, other: &Self) -> bool {
    self.cube_env == other.cube_env
  }
}

pub fn scene_gpus(
  scope: impl ReactiveCollection<AllocIdx<Scene>, ()>,
) -> impl ReactiveCollection<AllocIdx<Scene>, SceneGPUResource> {
}

impl SceneGPUResource {
  pub fn new(scene: &Scene, gpu: &GPU) -> Self {
    // let scene = scene.read().core.clone();
    // scene.single_listen_by(with_field_expand!(SceneCoreImpl => background));

    todo!()
  }
}

// AllocId<SceneModel> -> AllocId<Scene>
// AllocId<Scene> -m-> AllocId<SceneModel>
// multi relation filter by key set AllocId<Scene> -m-> AllocId<SceneModel>
// aka: AllocId<SceneModel> -> AllocId<Scene>
// drop value AllocId<SceneModel> ->()

// pub fn scenes_keep_gpu_resource(scope: impl ReactiveCollection<AllocIdx<Scene>, ()>) {
//   storage_of::<SceneModel>()
// }

struct SceneShareContentGPUResource {
  meshes: MeshGPUResource,
  materials: MaterialGPUResource,
  textures: TextureGPUResource,
}

fn create_scene_share_content_gpu_resource(
  scene_models: impl ReactiveCollection<AllocIdx<SceneModelImpl>, ()>,
) -> SceneShareContentGPUResource {
  let referenced_std_md =
    scene_models.many_to_one_reduce_key(scene_model_ref_std_model_many_one_relation());

  let referenced_attribute_mesh =
    referenced_std_md.many_to_one_reduce_key(std_model_ref_att_mesh());

  let referenced_attribute_mesh =
    referenced_std_md.many_to_one_reduce_key(std_model_ref_att_mesh());

  let referenced_flat_material =
    referenced_std_md.many_to_one_reduce_key(global_material_relations::<FlatMaterial>());

  SceneShareContentGPUResource {
    meshes: MeshGPUResource {},
    materials: MaterialGPUResource {},
    textures: todo!(),
  }
}

struct MeshGPUResource {}

struct MaterialGPUResource {
  flat_material_uniforms:
    Box<dyn ReactiveCollection<AllocIdx<FlatMaterial>, UniformBufferDataView<FlatMaterialUniform>>>,
  mr_material_uniforms: Box<
    dyn ReactiveCollection<
      AllocIdx<PhysicalMetallicRoughnessMaterial>,
      UniformBufferDataView<PhysicalMetallicRoughnessMaterialUniform>,
    >,
  >,
  mr_material_tex_uniforms: Box<
    dyn ReactiveCollection<
      AllocIdx<PhysicalMetallicRoughnessMaterial>,
      UniformBufferDataView<PhysicalMetallicRoughnessMaterialTextureHandlesUniform>,
    >,
  >,
}

struct TextureGPUResource {
  pub texture2ds: RxCForker<AllocIdx<SceneTexture2DType>, Texture2DHandle>,
  pub samplers: RxCForker<AllocIdx<TextureSampler>, SamplerHandle>,
}
