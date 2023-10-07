use crate::*;

const LIGHT_MAX: usize = 8;

pub struct LightList<T: ShaderLight> {
  uniform: ClampedUniformList<T, LIGHT_MAX>,
  empty_list: Vec<usize>,
  // map light id to index
  mapping: FastHashMap<u64, usize>,
  gpu: ResourceGPUCtx,
}

impl<T: ShaderLight> LightList<T> {
  pub fn new(gpu: ResourceGPUCtx) -> Self {
    Self {
      uniform: Default::default(),
      empty_list: (0..LIGHT_MAX).rev().collect(),
      mapping: Default::default(),
      gpu,
    }
  }

  pub fn update(&mut self, light_id: u64, light: Option<T>) {
    if let Some(value) = light {
      if let Some(idx) = self.mapping.get(&light_id) {
        self.uniform.source[*idx] = value;
      } else {
        let idx = self.empty_list.pop().unwrap();
        self.mapping.insert(light_id, idx);
        while self.uniform.source.len() <= idx {
          self.uniform.source.push(T::default());
        }
        self.uniform.source[idx] = value;
      }
    } else {
      let idx = self.mapping.remove(&light_id).unwrap();
      self.empty_list.push(idx);
    }
  }

  pub fn maintain(&mut self) -> usize {
    let empty_size = self.empty_list.len();

    self.empty_list.sort_by(|a, b| b.cmp(a));
    // compact empty slot, todo, reduce data movement
    let mut i = LIGHT_MAX;
    for empty_index in &mut self.empty_list {
      i -= 1;
      if *empty_index != i {
        self.uniform.source[*empty_index] = self.uniform.source[i];
        *empty_index = i;
      }
    }

    self.uniform.update_gpu(&self.gpu.device);
    LIGHT_MAX - empty_size
  }
}

impl<T: ShaderLight> ShaderHashProvider for LightList<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.uniform.hash_pipeline(hasher)
  }
}
impl<T: ShaderLight> ShaderPassBuilder for LightList<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.uniform.setup_pass(ctx)
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.uniform.post_setup_pass(ctx)
  }
}

impl<T: ShaderLight> LightCollectionCompute for LightList<T> {
  fn compute_lights(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let lights: UniformNode<_> = binding.bind_by(self.uniform.gpu.as_ref().unwrap());

    T::create_dep(builder);

    let light_specular_result = val(Vec3::zero()).make_local_var();
    let light_diffuse_result = val(Vec3::zero()).make_local_var();

    let light_count = builder.query::<LightCount>().unwrap();

    lights
      .into_shader_iter()
      .clamp_by(light_count)
      .for_each(|(_, light), _| {
        let light = light.load().expand();
        let light_result =
          T::compute_direct_light(builder, &light, geom_ctx, shading_impl, shading);

        // improve impl by add assign
        light_specular_result.store(light_specular_result.load() + light_result.specular);
        light_diffuse_result.store(light_diffuse_result.load() + light_result.diffuse);
      });

    ENode::<ShaderLightingResult> {
      diffuse: light_diffuse_result.load(),
      specular: light_specular_result.load(),
    }
  }
}
