use crate::*;

pub trait DrawUnitWorldTransformProvider: ShaderHashProvider {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn DrawUnitWorldTransformInvocationProvider>;
  fn bind(&self, cx: &mut BindingBuilder);
}

pub trait DrawUnitWorldTransformInvocationProvider {
  fn get_world_matrix(&self, id: Node<u32>) -> (Node<Mat4<f32>>, Node<HighPrecisionTranslation>);
}

pub fn use_scene_model_device_world_transform(
  cx: &mut QueryGPUHookCx,
) -> Option<DrawUnitWorldTransformProviderDefaultImpl> {
  let (cx, storage) = cx.use_storage_buffer(128, u32::MAX);

  cx.use_shared_dual_query(GlobalSceneModelWorldMatrix)
    .into_delta_change()
    .map(|v| {
      v.collective_map(|mat| {
        let (mat, position) = into_mat_hpt_storage_pair(mat);
        WorldMatrixStorage {
          matrix_none_translation: mat,
          position,
          ..Default::default()
        }
      })
    })
    .use_assure_result(cx)
    .update_storage_array(storage, 0);

  cx.when_render(|| DrawUnitWorldTransformProviderDefaultImpl {
    bounding_storage: storage.get_gpu_buffer(),
  })
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct WorldMatrixStorage {
  // todo use mat3
  pub matrix_none_translation: Mat4<f32>,
  pub position: HighPrecisionTranslationStorage,
}

#[derive(Clone)]
pub struct DrawUnitWorldTransformProviderDefaultImpl {
  bounding_storage: StorageBufferReadonlyDataView<[WorldMatrixStorage]>,
}

impl ShaderHashProvider for DrawUnitWorldTransformProviderDefaultImpl {
  shader_hash_type_id! {}
}
impl DrawUnitWorldTransformProvider for DrawUnitWorldTransformProviderDefaultImpl {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn DrawUnitWorldTransformInvocationProvider> {
    Box::new(DrawUnitWorldTransformInvocationProviderDefaultImpl {
      bounding_storage: cx.bind_by(&self.bounding_storage),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.bounding_storage);
  }
}

struct DrawUnitWorldTransformInvocationProviderDefaultImpl {
  bounding_storage: ShaderReadonlyPtrOf<[WorldMatrixStorage]>,
}

impl DrawUnitWorldTransformInvocationProvider
  for DrawUnitWorldTransformInvocationProviderDefaultImpl
{
  fn get_world_matrix(&self, id: Node<u32>) -> (Node<Mat4<f32>>, Node<HighPrecisionTranslation>) {
    let transform = self.bounding_storage.index(id).load().expand();
    (
      transform.matrix_none_translation,
      hpt_storage_to_hpt(transform.position),
    )
  }
}
