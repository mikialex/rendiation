use std::{cell::Cell, hash::Hash};

use rendiation_webgpu::*;

use crate::*;

#[derive(Clone)]
pub struct SceneMaterial<T> {
  pub material: T,
  pub states: MaterialStates,
}

pub trait IntoCommonSceneMaterial: Sized {
  fn into_scene_material(self) -> SceneMaterial<Self> {
    SceneMaterial {
      material: self,
      states: Default::default(),
    }
  }
}

impl<T> IntoCommonSceneMaterial for T {}

impl<T: MaterialCPUResource> ShaderGraphProvider for SceneMaterialGPU<T> {}

// impl<T> MaterialGPUResource for SceneMaterialGPU<T>
// where
//   T: MaterialGPUResource,
// {
//   type Source = SceneMaterial<T::Source>;

//   fn create_pipeline(
//     &self,
//     source: &Self::Source,
//     builder: &mut PipelineBuilder,
//     device: &wgpu::Device,
//     ctx: &PipelineCreateCtx,
//   ) {
//     source
//       .states
//       .apply_pipeline_builder(builder, &ctx.pass_info.format_info);

//     builder.with_layout::<TransformGPU>(ctx.layouts, device);

//     builder
//       .include_vertex_entry(
//         "
//     [[stage(vertex)]]
//       fn vs_main(
//         [[location(0)]] position: vec3<f32>, // todo link with vertex type
//         [[location(1)]] normal: vec3<f32>,
//         [[location(2)]] uv: vec2<f32>,
//       ) -> VertexOutput {
//         var out: VertexOutput;
//         out.uv = uv;
//         out.position = camera.projection * camera.view * model.matrix * vec4<f32>(position, 1.0);;
//         return out;
//       }

//     ",
//       )
//       .declare_io_struct(
//         "
//       struct VertexOutput {
//         [[builtin(position)]] position: vec4<f32>;
//         [[location(0)]] uv: vec2<f32>;
//       };
//     ",
//       )
//       .use_vertex_entry("vs_main");

//     self
//       .gpu
//       .create_pipeline(&source.material, builder, device, ctx);

//     builder.with_layout::<CameraBindgroup>(ctx.layouts, device);
//   }

//   fn setup_pass_bindgroup<'a>(
//     &self,
//     pass: &mut GPURenderPass<'a>,
//     ctx: &SceneMaterialPassSetupCtx,
//   ) {
//     pass.set_bind_group_owned(0, &ctx.model_gpu.unwrap().bindgroup, &[]);
//     self.gpu.setup_pass_bindgroup(pass, ctx);
//     pass.set_bind_group_owned(2, &ctx.camera_gpu.bindgroup, &[]);
//   }
// }

pub struct SceneMaterialGPU<T> {
  state_id: Cell<ValueID<MaterialStates>>,
  gpu: T,
}

impl<T> MaterialCPUResource for SceneMaterial<T>
where
  T: Clone,
  T: MaterialCPUResource,
{
  type GPU = SceneMaterialGPU<T::GPU>;

  fn create_gpu(&self, ctx: &mut SceneMaterialRenderPrepareCtx) -> Self::GPU {
    let gpu = self.material.create(ctx);

    let state_id = STATE_ID.lock().unwrap().get_uuid(&self.states);

    SceneMaterialGPU {
      state_id: Cell::new(state_id),
      gpu,
    }
  }

  fn hash_pipeline(&self, hasher: &mut PipelineHasher, gpu: &Self::GPU) {
    gpu
      .state_id
      .set(STATE_ID.lock().unwrap().get_uuid(&self.states));
    gpu.state_id.get().hash(hasher);
    self.material.hash_pipeline(&gpu.gpu, hasher);
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.material.is_keep_mesh_shape()
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
