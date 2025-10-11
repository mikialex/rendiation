use std::sync::Arc;

use fast_hash_collection::{FastHashMap, FastHashSet};
use parking_lot::RwLock;
use rendiation_texture_core::{GPUBufferImage, Size};
use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

pub struct BoneMatrixProvider {
  data: GPU2DTextureView,
}

pub struct BoneMatrixInvocationProvider {
  data: BindingNode<ShaderTexture2D>,
}
impl BoneMatrixAccessInvocation for BoneMatrixInvocationProvider {
  fn get_matrix(&self, joint_index: Node<u32>) -> Node<Mat4<f32>> {
    let joint_index = joint_index * val(4);
    let uv = vec2_node((joint_index, val(0)));
    let m1 = self.data.load_texel(uv, val(0));

    let uv = vec2_node((joint_index + val(1), val(0)));
    let m2 = self.data.load_texel(uv, val(0));

    let uv = vec2_node((joint_index + val(2), val(0)));
    let m3 = self.data.load_texel(uv, val(0));

    let uv = vec2_node((joint_index + val(3), val(0)));
    let m4 = self.data.load_texel(uv, val(0));

    (m1, m2, m3, m4).into()
  }
}

impl GraphicsShaderProvider for BoneMatrixProvider {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, bind| {
      let bone_data = bind.bind_by(&self.data);
      let bone_impl = BoneMatrixInvocationProvider { data: bone_data };
      let bone_impl = Box::new(bone_impl) as Box<dyn BoneMatrixAccessInvocation>;
      builder.registry_any_map().register(bone_impl);
    })
  }
}
impl ShaderPassBuilder for BoneMatrixProvider {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.data);
  }
}
impl ShaderHashProvider for BoneMatrixProvider {
  shader_hash_type_id! {}
}

pub fn use_skin(cx: &mut QueryGPUHookCx) -> Option<LockReadGuardHolder<SkinBoneMatrixesGPU>> {
  let mat_updates = use_indexed_joints_offset_mats(cx).use_assure_result(cx);

  // todo, use entity set changes
  let skin_change = cx
    .use_dual_query_set::<SceneSkinEntity>()
    .use_assure_result(cx);

  let (cx, skin_gpu) = cx.use_plain_state_default::<SharedSkinBoneMatrixesGPU>();

  cx.when_render(|| {
    let mut skin_gpu_ = skin_gpu.write();
    let mat_updates = mat_updates.expect_resolve_stage().delta().into_change();
    let skin_change = skin_change.expect_resolve_stage().delta().into_change();
    let removed_skin = skin_change.iter_removed();

    skin_gpu_.update(mat_updates, removed_skin, cx.gpu);
    drop(skin_gpu_);

    skin_gpu.make_read_holder()
  })
}

pub type SharedSkinBoneMatrixesGPU = Arc<RwLock<SkinBoneMatrixesGPU>>;

/// in gles mode we have to use texture to store bone matrix
#[derive(Default)]
pub struct SkinBoneMatrixesGPU {
  bind_matrixes: FastHashMap<RawEntityHandle, (Vec<Mat4<f32>>, Option<GPU2DTextureView>)>,
}

impl SkinBoneMatrixesGPU {
  pub fn get_bone_provider(
    &self,
    skin: EntityHandle<SceneSkinEntity>,
  ) -> Option<BoneMatrixProvider> {
    self
      .bind_matrixes
      .get(&skin.into_raw())
      .and_then(|(_, gpu_texture)| {
        let data = gpu_texture.clone()?;
        BoneMatrixProvider { data }.into()
      })
  }
  pub fn update(
    &mut self,
    mat_changes: impl DataChanges<Key = RawEntityHandle, Value = (Mat4<f32>, u32)>, /* EntityHandle<SceneJointEntity> */
    removed_skins: impl Iterator<Item = RawEntityHandle>, // EntityHandle<SceneSkinEntity>
    gpu: &GPU,
  ) {
    let skin_access = get_db_view::<SceneJointBelongToSkin>();
    for k in removed_skins {
      self.bind_matrixes.remove(&k);
    }

    let mut changed_skins = FastHashSet::default();

    for (k, (value, idx)) in mat_changes.iter_update_or_insert() {
      let skin = skin_access.access(&k).unwrap().unwrap();
      let (bind_matrixes, _) = self.bind_matrixes.entry(skin).or_default();
      bind_matrixes.resize(
        bind_matrixes.len().max((idx + 1) as usize),
        Mat4::identity(),
      );
      bind_matrixes[idx as usize] = value;
      changed_skins.insert(skin);
    }

    for skin in changed_skins {
      let (bind_matrixes, gpu_texture) = self.bind_matrixes.get_mut(&skin).unwrap();
      *gpu_texture = Some(create_data_texture(gpu, bind_matrixes));
    }
  }
}

fn create_data_texture(cx: &GPU, bind_matrixes: &[Mat4<f32>]) -> GPU2DTextureView {
  let pixel_count_required = bind_matrixes.len() * 4;
  let image = GPUBufferImage {
    data: cast_slice(bind_matrixes).to_vec(),
    format: TextureFormat::Rgba32Float,
    size: Size::from_usize_pair_min_one((pixel_count_required, 1)),
  };
  let texture = GPUBufferImageForeignImpl { inner: &image };

  let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap, cx.info().downgrade_info.flags);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);
  gpu_texture.create_default_view().try_into().unwrap()
}
