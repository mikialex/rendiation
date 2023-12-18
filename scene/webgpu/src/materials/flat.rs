use crate::*;

pub fn flat_material_gpus(
  cx: &ResourceGPUCtx,
) -> impl ReactiveCollection<AllocIdx<FlatMaterial>, FlatMaterialGPU> {
  let cx = cx.clone();
  storage_of::<FlatMaterial>()
    .listen_to_reactive_collection(|_| Some(()))
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let creator = storage_of::<FlatMaterial>().create_key_mapper(move |m| {
        let cx = cx.clone();

        let uniform = FlatMaterialUniform {
          color: srgba_to_linear(m.color),
          ..Zeroable::zeroed()
        };
        let uniform = create_uniform(uniform, &cx.device);
        FlatMaterialGPU { uniform }
      });
      move |k, _| creator(*k)
    })
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

#[derive(Clone)]
pub struct FlatMaterialGPU {
  uniform: UniformBufferDataView<FlatMaterialUniform>,
}

impl ShaderHashProvider for FlatMaterialGPU {}

impl GraphicsShaderProvider for FlatMaterialGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();

      builder.register::<DefaultDisplay>(uniform.color);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
  }
}

fn create_flat_material_uniform(m: &FlatMaterial) -> FlatMaterialUniform {
  FlatMaterialUniform {
    color: srgba_to_linear(m.color),
    ..Zeroable::zeroed()
  }
}
