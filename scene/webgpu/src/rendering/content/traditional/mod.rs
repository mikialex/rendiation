use crate::*;

mod list;
pub use list::*;

impl SceneRenderable for SceneModel {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    self.visit(|model| model.render(pass, dispatcher, camera, scene))
  }
}

impl SceneRenderable for SceneModelImpl {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    setup_pass_core(self, pass, camera, None, dispatcher, scene);
  }
}

pub fn setup_pass_core(
  model_input: &SceneModelImpl,
  pass: &mut FrameRenderPass,
  camera: &SceneCamera,
  override_node: Option<&NodeGPU>,
  dispatcher: &dyn RenderComponentAny,
  resources: &SceneRenderResourceGroup,
) {
  match &model_input.model {
    ModelEnum::Standard(model) => {
      let model = model.read();
      let pass_gpu = dispatcher;

      let cameras = resources.scene_resources.cameras.read().unwrap();
      let camera_gpu = cameras.get_camera_gpu(camera).unwrap();

      let net_visible = resources.node_derives.get_net_visible(&model_input.node);
      if !net_visible {
        return;
      }

      let node_gpu = override_node.unwrap_or(
        resources
          .scene_resources
          .nodes
          .get_node_gpu(&model_input.node)
          .unwrap(),
      );

      let mut materials = resources.resources.model_ctx.materials.write().unwrap();
      let material_gpu = materials.get_or_insert_with(model.material.guid().unwrap(), || {
        model
          .material
          .create_scene_reactive_gpu(&resources.resources.bindable_ctx)
          .unwrap()
      });

      let mut meshes = resources.resources.model_ctx.meshes.write().unwrap();
      if model.mesh.guid().is_none() {
        model.mesh.guid().unwrap();
      }
      let mesh_gpu = meshes.get_or_insert_with(model.mesh.guid().unwrap(), || {
        model
          .mesh
          .create_scene_reactive_gpu(&resources.resources.bindable_ctx)
          .unwrap()
      });

      let draw_command = mesh_gpu.draw_command(model.group);

      dispatch_model_draw_with_preferred_binding_frequency(
        pass_gpu,
        mesh_gpu,
        node_gpu,
        camera_gpu,
        material_gpu,
        draw_command,
        &mut pass.ctx,
      );
    }
    ModelEnum::Foreign(_) => {
      todo!()
      // if let Some(model) = model.downcast_ref::<Box<dyn SceneRenderable>>() {
      //   model.render(pass, dispatcher, camera, resources)
      // }
    }
  };
}

pub fn dispatch_model_draw_with_preferred_binding_frequency(
  base: &dyn RenderComponentAny,
  mesh: &MeshGPUInstance,
  node: &NodeGPU,
  camera: &CameraGPU,
  material: &MaterialGPUInstance,
  draw_command: DrawCommand,
  pass: &mut GPURenderPassCtx,
) {
  let components: [&dyn RenderComponentAny; 5] = [
    &base.assign_binding_index(0),
    &mesh.assign_binding_index(2),
    &node.assign_binding_index(2),
    &camera.assign_binding_index(1),
    &material.assign_binding_index(2),
  ];

  RenderEmitter::new(components.as_slice()).render(pass, draw_command);
}
