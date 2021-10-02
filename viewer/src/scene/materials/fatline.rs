pub struct FatLineMaterial {
  width: f32,
}

pub struct FatLineMaterialGPU {
  uniform: UniformBuffer<f32>,
  bindgroup: BindGroup,
}

impl MaterialMeshLayoutRequire for BasicMaterial {
  type VertexInput = Vec<FatLineVertex>;
}

pub struct FatLineVertex {
  position: Vec3<f32>,
  color: Vec3<f32>,
}

pub struct BasicMaterialGPU {
  state_id: ValueID<MaterialStates>,
  _uniform: UniformBuffer<Vec3<f32>>,
  bindgroup: MaterialBindGroup,
}

impl MaterialGPUResource for BasicMaterialGPU {
  type Source = BasicMaterial;

  fn request_pipeline(
    &mut self,
    source: &Self::Source,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) {
    self.state_id = STATE_ID.lock().unwrap().get_uuid(source.states);

    let key = CommonPipelineVariantKey(self.state_id, ctx.active_mesh.unwrap().topology());

    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();

    pipelines
      .get_cache_mut::<Self, CommonPipelineCache>()
      .request(&key, || source.create_pipeline(gpu, &pipeline_ctx));
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    let key = CommonPipelineVariantKey(self.state_id, ctx.active_mesh.unwrap().topology());

    let pipeline = ctx
      .pipelines
      .get_cache::<Self, CommonPipelineCache>()
      .retrieve(&key);

    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &ctx.model_gpu.unwrap().bindgroup, &[]);
    pass.set_bind_group(1, &self.bindgroup.gpu, &[]);
    pass.set_bind_group(2, &ctx.camera_gpu.bindgroup, &[]);
  }
}

impl MaterialCPUResource for BasicMaterial {
  type GPU = BasicMaterialGPU;

  fn create(
    &mut self,
    handle: MaterialHandle,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, self.color);

    let bindgroup_layout = Self::create_bindgroup_layout(&gpu.device);
    let bindgroup =
      self.create_bindgroup(handle, _uniform.gpu(), &gpu.device, &bindgroup_layout, ctx);

    let state_id = STATE_ID.lock().unwrap().get_uuid(self.states);

    BasicMaterialGPU {
      state_id,
      _uniform,
      bindgroup,
    }
  }
}
