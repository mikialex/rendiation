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
