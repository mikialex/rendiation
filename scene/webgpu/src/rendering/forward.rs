use crate::*;

pub fn get_main_pass_load_op<S>(scene: &Scene<S>) -> webgpu::Operations<webgpu::Color>
where
  S: SceneContent,
  S::BackGround: Deref<Target = dyn WebGPUBackground>,
{
  let load = if let Some(clear_color) = scene.background.as_ref().unwrap().require_pass_clear() {
    webgpu::LoadOp::Clear(clear_color)
  } else {
    webgpu::LoadOp::Load
  };

  webgpu::Operations { load, store: true }
}

pub struct ForwardScene;

impl<S> PassContentWithSceneAndCamera<S> for ForwardScene
where
  S: SceneContent,
  S::Model: Deref<Target = dyn SceneModelShareable>,
{
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera) {
    let mut render_list = RenderList::<S>::default();
    render_list.prepare(scene, camera);
    render_list.setup_pass(pass, scene, &pass.default_dispatcher(), camera);
  }
}

/// contains gpu data that support forward rendering
pub struct ForwardLightingSystem {
  pub lights_collections: HashMap<TypeId, Box<dyn LightCollectionCompute>>,
}

impl ForwardLightingSystem {
  pub fn update_by_scene(&mut self, scene: &Scene<WebGPUScene>) {
    for (_, light) in &scene.lights {
      let light = &light.read().light;

      //
    }
  }

  pub fn compute_lights(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let camera_position = builder.query::<FragmentWorldPosition>()?.get(); // todo
      let geom_position = builder.query::<FragmentWorldPosition>()?.get();

      let geom_ctx = ExpandedNode::<ShaderLightingGeometricCtx> {
        position: geom_position,
        normal: builder.query::<FragmentWorldNormal>()?.get(),
        view_dir: camera_position - geom_position,
      };
      let shading = shading_impl.construct_shading_dyn(builder);

      let mut light_specular_result = consts(Vec3::zero());
      let mut light_diffuse_result = consts(Vec3::zero());

      for lights in self.lights_collections.values() {
        let (diffuse, specular) =
          lights.compute_lights(builder, binding, shading_impl, &shading, &geom_ctx)?;
        light_specular_result = specular + light_specular_result;
        light_diffuse_result = diffuse + light_diffuse_result;
      }

      let hdr_result = ExpandedNode::<ShaderLightingResult> {
        diffuse: light_diffuse_result,
        specular: light_specular_result,
      }
      .construct();

      builder.register::<HDRLightResult>(hdr_result);

      Ok(())
    })
  }
}

pub struct LightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: UniformBufferDataView<Shader140Array<T, 32>>,
}

impl<T: ShaderLight> LightList<T> {
  pub fn update(&mut self) {
    //
  }
}

impl<T: ShaderLight> ShaderPassBuilder for LightList<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.lights_gpu, SB::Pass);
  }
}

pub trait LightCollectionCompute: ShaderPassBuilder {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError>;

  fn compute_lights_grouped(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<ExpandedNode<ShaderLightingResult>, ShaderGraphBuildError> {
    let (diffuse, specular) =
      self.compute_lights(builder, binding, shading_impl, shading, geom_ctx)?;
    Ok(ExpandedNode::<ShaderLightingResult> { diffuse, specular })
  }
}

impl<T: ShaderLight> LightCollectionCompute for LightList<T> {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError> {
    let lights = binding.uniform_by(&self.lights_gpu, SB::Pass);

    let dep = T::create_dep(builder);

    let light_specular_result = consts(Vec3::zero()).mutable();
    let light_diffuse_result = consts(Vec3::zero()).mutable();

    for_by(lights, |_, light| {
      let light = light.expand();
      let incident = T::compute_direct_light(&light, &dep, geom_ctx);
      let light_result = shading_impl.compute_lighting_dyn(shading, &incident, geom_ctx);

      // improve impl by add assign
      light_specular_result.set(light_specular_result.get() + light_result.specular);
      light_diffuse_result.set(light_diffuse_result.get() + light_result.diffuse);
    });

    Ok((light_diffuse_result.get(), light_specular_result.get()))
  }
}
