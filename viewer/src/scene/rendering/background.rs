pub struct BackGroundRendering;

impl PassContent for BackGroundRendering {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, resource: &mut ResourcePoolInner) {
    if let Some(active_camera) = &mut scene.active_camera {
      let camera_gpu = scene
        .active_camera_gpu
        .get_or_insert_with(|| CameraBindgroup::new(gpu))
        .update(gpu, active_camera, &scene.nodes);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass: todo!(),
        pipelines: &mut scene.pipeline_resource,
        layouts: &mut scene.layouts,
        textures: &mut scene.texture_2ds,
        texture_cubes: &mut scene.texture_cubes,
        samplers: &mut scene.samplers,
        reference_finalization: &scene.reference_finalization,
      };

      scene.background.update(
        gpu,
        &mut base,
        &mut scene.materials,
        &mut scene.meshes,
        &mut scene.nodes,
      );
    }
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    resource: &'a ResourcePoolInner,
  ) {
    scene.background.setup_pass(
      pass,
      &scene.materials,
      &scene.meshes,
      &scene.nodes,
      scene.active_camera_gpu.as_ref().unwrap(),
      &scene.pipeline_resource,
      todo!(),
    );
  }
  //
}
