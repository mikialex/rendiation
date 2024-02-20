use crate::*;

#[derive(Clone)]
pub struct SceneGPUResource {
  // exist if scene is env background
  pub(crate) cube_env: Option<GPUCubeTextureView>,
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

pub struct SceneShareContentGPUResource {
  cameras: Box<dyn ReactiveCollection<SceneCameraImpl, CameraGPU>>,
  attributes_meshes: AttributesMeshGPUResource,
  materials: MaterialGPUResource,
  textures: TextureGPUResource,
}

fn create_scene_model_pipeline_hash(
  scene_sm_scope: impl ReactiveCollection<AllocIdx<SceneModelImpl>, ()>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, u64> {
}

fn create_scene_share_content_gpu_resource(
  cx: &ResourceGPUCtx,
  all_scene_sm_scope: impl ReactiveCollection<AllocIdx<SceneModelImpl>, ()>,
) -> SceneShareContentGPUResource {
  let referenced_std_md = all_scene_sm_scope
    .many_to_one_reduce_key(scene_model_ref_std_model_many_one_relation())
    .into_forker();

  let texture_material_references = all_std_model_materials_textures(referenced_std_md.clone());

  let all_material_tex_ref_path = texture_material_references.normalized_path().into_forker();

  // todo, this looks awful, could we fix the one to many relation directly?
  let all_material_reference_tex_set = all_material_tex_ref_path
    .clone()
    .collective_map(|_| ())
    .many_to_one_reduce_key(all_material_tex_ref_path);

  let texture2ds = gpu_texture_2ds(cx, all_material_reference_tex_set)
    .collective_map(|_| todo!())
    .into_boxed()
    .into_forker();

  let bindless_texture2ds: RxCForker<AllocIdx<SceneTexture2DType>, TextureSamplerHandlePair> =
    todo!();

  let referenced_attribute_mesh = referenced_std_md
    .clone()
    .many_to_one_reduce_key(std_model_ref_att_mesh())
    .into_forker();

  let referenced_flat_material = referenced_std_md
    .clone()
    .many_to_one_reduce_key(global_material_relations::<FlatMaterial>());
  let flat_material_uniforms =
    flat_material_gpus(cx.clone(), referenced_flat_material).into_boxed();

  let referenced_mr_material = referenced_std_md
    .clone()
    .many_to_one_reduce_key(global_material_relations::<PhysicalMetallicRoughnessMaterial>());
  let mr_material_uniforms =
    physical_mr_material_uniforms(cx.clone(), referenced_mr_material).into_boxed();
  let mr_material_tex_uniforms = PhysicalMetallicRoughnessMaterial::create_texture_uniforms(
    texture_material_references.mr.clone(),
    bindless_texture2ds.clone(),
  )
  .into_boxed();

  let referenced_sg_material =
    referenced_std_md
      .clone()
      .many_to_one_reduce_key(global_material_relations::<
        PhysicalSpecularGlossinessMaterial,
      >());
  let sg_material_uniforms =
    physical_sg_material_uniforms(cx.clone(), referenced_sg_material).into_boxed();
  let sg_material_tex_uniforms = PhysicalSpecularGlossinessMaterial::create_texture_uniforms(
    texture_material_references.sg.clone(),
    bindless_texture2ds,
  )
  .into_boxed();

  let vertex_buffers = vertex_attribute_buffers_scope(referenced_attribute_mesh.clone());
  let vertex_buffers = gpu_attribute_vertex_buffers(cx, vertex_buffers).into_boxed();

  let index_buffers = vertex_attribute_buffers_scope(referenced_attribute_mesh);
  let index_buffers = gpu_attribute_index_buffers(cx, index_buffers).into_boxed();

  SceneShareContentGPUResource {
    attributes_meshes: AttributesMeshGPUResource {
      vertex_buffers,
      index_buffers,
    },
    materials: MaterialGPUResource {
      flat_material_uniforms,
      mr_material_uniforms,
      mr_material_tex_uniforms,
      sg_material_uniforms,
      sg_material_tex_uniforms,
    },
    textures: TextureGPUResource {
      texture2ds,
      samplers: sampler_gpus_handles(cx, todo!()).into_boxed().into_forker(),
    },
    cameras: todo!(),
  }
}

struct AttributesMeshGPUResource {
  vertex_buffers: Box<dyn ReactiveCollection<AttributeAccessKey, GPUBufferResourceView>>,
  index_buffers: Box<dyn ReactiveCollection<AttributeAccessKey, GPUBufferResourceView>>,
}

type UniformCollection<K, V> = Box<dyn ReactiveCollection<AllocIdx<K>, UniformBufferDataView<V>>>;

struct MaterialGPUResource {
  flat_material_uniforms:
    Box<dyn ReactiveCollection<AllocIdx<FlatMaterial>, UniformBufferDataView<FlatMaterialUniform>>>,

  mr_material_uniforms:
    UniformCollection<PhysicalMetallicRoughnessMaterial, PhysicalMetallicRoughnessMaterialUniform>,

  mr_material_tex_uniforms: UniformCollection<
    PhysicalMetallicRoughnessMaterial,
    PhysicalMetallicRoughnessMaterialTextureHandlesUniform,
  >,

  sg_material_uniforms: UniformCollection<
    PhysicalSpecularGlossinessMaterial,
    PhysicalSpecularGlossinessMaterialUniform,
  >,

  sg_material_tex_uniforms: UniformCollection<
    PhysicalSpecularGlossinessMaterial,
    PhysicalSpecularGlossinessMaterialTextureHandlesUniform,
  >,
}

struct TextureGPUResource {
  pub texture2ds: RxCForker<AllocIdx<SceneTexture2DType>, Texture2DHandle>,
  pub samplers: RxCForker<AllocIdx<TextureSampler>, SamplerHandle>,
}
