mod directional;
pub use directional::*;
mod point;
pub use point::*;
mod spot;
pub use spot::*;

use crate::*;

pub struct AllInstanceOfGivenTypeLightInScene<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  pub light_accessor: MultiAccessGPUData,
  pub light_data: StorageBufferReadonlyDataView<[T]>,
  pub create_per_light_compute:
    std::sync::Arc<dyn Fn(ShaderReadonlyPtrOf<T>) -> Box<dyn LightingComputeInvocation>>,
}

/// create_per_light_compute is not hashed because we assume the implementation only
/// related to T type
impl<T> ShaderHashProvider for AllInstanceOfGivenTypeLightInScene<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  shader_hash_type_id! {}
}

impl<T: Std430 + ShaderSizedValueNodeType> LightingComputeComponent
  for AllInstanceOfGivenTypeLightInScene<T>
{
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    let compute = self.create_per_light_compute.clone();
    let lighting = AllInstanceOfGivenTypeLightInSceneInvocation {
      scene_id,
      light_accessor: self.light_accessor.build(binding),
      light_data: binding.bind_by(&self.light_data),
    }
    .map(move |ptr| compute(ptr));

    Box::new(ShaderIntoIterAsLightInvocation(lighting))
  }

  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.light_accessor.bind(&mut ctx.binding);
    ctx.binding.bind(&self.light_data);
  }
}

#[derive(Clone)]
pub struct AllInstanceOfGivenTypeLightInSceneInvocation<T: Std430> {
  pub scene_id: Node<u32>,
  pub light_accessor: MultiAccessGPUInvocation,
  pub light_data: ShaderReadonlyPtrOf<[T]>,
}

impl<T: Std430 + ShaderSizedValueNodeType> IntoShaderIterator
  for AllInstanceOfGivenTypeLightInSceneInvocation<T>
{
  type ShaderIter = impl ShaderIterator<Item = ShaderReadonlyPtrOf<T>>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    self
      .light_accessor
      .iter_refed_many_of(self.scene_id)
      .map(move |id| self.light_data.index(id))
  }
}
