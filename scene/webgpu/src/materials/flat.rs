use crate::*;

pub fn flat_material_gpus(
  cx: ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<FlatMaterial>, ()>,
) -> impl ReactiveCollectionSelfContained<
  AllocIdx<FlatMaterial>,
  UniformBufferDataView<FlatMaterialUniform>,
> {
  storage_of::<FlatMaterial>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(scope)
    .collective_create_uniforms_by_key(cx, |m| FlatMaterialUniform {
      color: srgba_to_linear(m.color),
      ..Zeroable::zeroed()
    })
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

#[derive(Clone)]
pub struct FlatMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<FlatMaterialUniform>,
}

impl<'a> ShaderHashProvider for FlatMaterialGPU<'a> {}

impl<'a> GraphicsShaderProvider for FlatMaterialGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();

      builder.register::<DefaultDisplay>(uniform.color);
      Ok(())
    })
  }
}

impl<'a> ShaderPassBuilder for FlatMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
  }
}

pub type UniformGetter<K, V> =
  Box<dyn VirtualCollectionSelfContained<AllocIdx<K>, UniformBufferDataView<V>>>;

pub struct FlatMaterialGPUResource {
  uniforms: UniformGetter<FlatMaterial, FlatMaterialUniform>,
}

impl FlatMaterialGPUResource {
  pub fn prepare_render(&self, flat: AllocIdx<FlatMaterial>) -> FlatMaterialGPU {
    FlatMaterialGPU {
      uniform: self.uniforms.access_ref(&flat).unwrap(),
    }
  }
}
