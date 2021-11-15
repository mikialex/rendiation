use std::{cell::Cell, rc::Rc};

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

pub struct SceneMaterialGPU<T> {
  state_id: Cell<ValueID<MaterialStates>>,
  gpu: T,
}

impl<T: MaterialGPUResource> PipelineRequester for SceneMaterialGPU<T> {
  type Container = CommonPipelineCache<T::Container>;
}

impl<T> MaterialGPUResource for SceneMaterialGPU<T>
where
  T: MaterialGPUResource,
{
  type Source = SceneMaterial<T::Source>;

  fn pipeline_key(
    &self,
    source: &Self::Source,
    ctx: &PipelineCreateCtx,
  ) -> <Self::Container as PipelineVariantContainer>::Key {
    self
      .state_id
      .set(STATE_ID.lock().unwrap().get_uuid(&source.states));
    self
      .gpu
      .pipeline_key(&source.material, ctx)
      .key_with(self.state_id.get())
      .key_with(ctx.active_mesh.unwrap().topology())
  }

  fn create_pipeline(
    &self,
    source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) {
    builder.targets = ctx
      .pass
      .format_info
      .color_formats
      .iter()
      .map(|&f| source.states.map_color_states(f))
      .collect();

    builder.depth_stencil = source
      .states
      .map_depth_stencil_state(ctx.pass.format_info.depth_stencil_format);

    builder.with_layout::<TransformGPU>(ctx.layouts, device);

    self
      .gpu
      .create_pipeline(&source.material, builder, device, ctx);

    builder.with_layout::<CameraBindgroup>(ctx.layouts, device);

    builder.primitive_state.topology = ctx.active_mesh.unwrap().topology();
  }

  fn setup_pass_bindgroup<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx,
  ) {
    pass.set_bind_group_owned(0, &ctx.model_gpu.unwrap().bindgroup, &[]);
    self.gpu.setup_pass_bindgroup(pass, ctx);
    pass.set_bind_group_owned(2, &ctx.camera_gpu.bindgroup, &[]);
  }
}

impl<T> MaterialCPUResource for SceneMaterial<T>
where
  T: Clone,
  T: MaterialCPUResource,
{
  type GPU = SceneMaterialGPU<T::GPU>;

  fn create(
    &mut self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let gpu = self.material.create(gpu, ctx, bgw);

    let state_id = STATE_ID.lock().unwrap().get_uuid(&self.states);

    SceneMaterialGPU {
      state_id: Cell::new(state_id),
      gpu,
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.material.is_keep_mesh_shape()
  }
}
