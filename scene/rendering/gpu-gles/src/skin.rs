use fast_hash_collection::FastHashMap;
use rendiation_texture_core::{GPUBufferImage, Size};
use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

pub trait BoneMatrixAccessInvocation {
  fn get_matrix(&self, joint_index: Node<u32>) -> Node<Mat4<f32>>;
}

pub struct SkinVertexTransform {
  pub skin_bind_mats: Option<UniformBufferDataView<SkinBindMatrixes>>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct SkinBindMatrixes {
  pub bind_mat: Mat4<f32>,
  pub inv_bind_mat: Mat4<f32>,
}

impl GraphicsShaderProvider for SkinVertexTransform {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, bind| {
      let position_pre_transform = builder.query::<GeometryPosition>();
      let normal_pre_transform = builder.query::<GeometryNormal>();
      let joints = builder.try_query::<JointIndexChannel<0>>();
      let weights = builder.try_query::<WeightChannel<0>>();

      let bone_mats = builder
        .registry()
        .any_map
        .get::<Box<dyn BoneMatrixAccessInvocation>>();

      if let (Some(joints), Some(weights), Some(bind_mats), Some(bone_mats)) =
        (joints, weights, &self.skin_bind_mats, bone_mats)
      {
        let bind_mats = bind.bind_by(bind_mats).load().expand();
        let bind_matrix = bind_mats.bind_mat;
        let bind_matrix_inverse = bind_mats.inv_bind_mat;

        let bone_matrix_x = bone_mats.get_matrix(joints.x());
        let bone_matrix_y = bone_mats.get_matrix(joints.y());
        let bone_matrix_z = bone_mats.get_matrix(joints.z());
        let bone_matrix_w = bone_mats.get_matrix(joints.w());
        //

        let pre_transform: Node<Vec4<_>> = (position_pre_transform, val(1.0)).into();
        let skin_vertex = bind_matrix * pre_transform;

        let skinned = bone_matrix_x * skin_vertex * weights.x();
        let skinned = skinned + bone_matrix_y * skin_vertex * weights.y();
        let skinned = skinned + bone_matrix_z * skin_vertex * weights.z();
        let skinned = skinned + bone_matrix_w * skin_vertex * weights.w();

        let position_transformed = (bind_matrix_inverse * skinned).xyz();
        builder.register::<GeometryPosition>(position_transformed);

        let skin_matrix = weights.x() * bone_matrix_x;
        let skin_matrix = skin_matrix + weights.y() * bone_matrix_y;
        let skin_matrix = skin_matrix + weights.z() * bone_matrix_z;
        let skin_matrix = skin_matrix + weights.w() * bone_matrix_w;
        let skin_matrix = bind_matrix_inverse * skin_matrix * bind_matrix;

        let normal_pre_transform = (normal_pre_transform, val(1.0)).into();
        let normal_transformed = (skin_matrix * normal_pre_transform).xyz();
        builder.register::<GeometryNormal>(normal_transformed);
      }
    })
  }
}

pub struct BoneMatrixProvider {
  data: GPU2DTextureView,
}

pub struct BoneMatrixInvocationProvider {
  data: BindingNode<ShaderTexture2D>,
}
impl BoneMatrixAccessInvocation for BoneMatrixInvocationProvider {
  fn get_matrix(&self, joint_index: Node<u32>) -> Node<Mat4<f32>> {
    let uv = vec2_node((joint_index, val(0)));
    let m1 = self.data.load_texel(uv, val(0));

    let uv = vec2_node((joint_index + val(1), val(0)));
    let m2 = self.data.load_texel(uv, val(0));

    let uv = vec2_node((joint_index + val(2), val(1)));
    let m3 = self.data.load_texel(uv, val(0));

    let uv = vec2_node((joint_index + val(3), val(1)));
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

/// in gles mode we have to use texture to store bone matrix
pub struct SkinBoneMatrixesDataTextureComputer {
  bind_matrixes:
    FastHashMap<EntityHandle<SceneSkinEntity>, (Vec<Mat4<f32>>, Option<GPU2DTextureView>)>,
}

impl SkinBoneMatrixesDataTextureComputer {
  pub fn get_bone_provider(
    &self,
    skin: EntityHandle<SceneSkinEntity>,
  ) -> Option<BoneMatrixProvider> {
    self.bind_matrixes.get(&skin).and_then(|(_, gpu_texture)| {
      let data = gpu_texture.clone()?;
      BoneMatrixProvider { data }.into()
    })
  }
  pub fn update(
    &mut self,
    mat_changes: impl DataChanges<Key = EntityHandle<SceneJointEntity>, Value = (Mat4<f32>, u32)>,
    removed_skins: impl Iterator<Item = EntityHandle<SceneSkinEntity>>,
    gpu: &GPU,
  ) {
    let skin_access = global_database().read_foreign_key::<SceneJointBelongToSkin>();
    for k in removed_skins {
      self.bind_matrixes.remove(&k);
    }

    for (k, (value, idx)) in mat_changes.iter_update_or_insert() {
      let skin = skin_access.get(k).unwrap();
      let (bind_matrixes, gpu) = self.bind_matrixes.entry(skin).or_default();
      bind_matrixes.resize(bind_matrixes.len().max(idx as usize), Mat4::identity());
      bind_matrixes[idx as usize] = value;
      *gpu = None;
    }

    for (k, (change, idx)) in mat_changes.iter_update_or_insert() {
      let skin = skin_access.get(k).unwrap();
      let (bind_matrixes, gpu_texture) = self.bind_matrixes.get_mut(&skin).unwrap();
      let idx = idx as usize;
      bind_matrixes.resize(idx, Mat4::identity());
      bind_matrixes[idx] = change;

      gpu_texture.get_or_insert_with(|| create_data_texture(gpu, bind_matrixes));
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

  let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);
  gpu_texture.create_default_view().try_into().unwrap()
}
