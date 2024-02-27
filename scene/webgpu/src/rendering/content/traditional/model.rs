use crate::*;

// pub trait SceneRenderable {
//   fn render(
//     &self,
//     pass: &mut FrameRenderPass,
//     dispatcher: &dyn RenderComponentAny,
//     camera: &SceneCamera,
//     scene: &SceneRenderResourceGroup,
//   );
// }

pub struct SceneModelGPUResource {
  sm: StorageReadView<SceneModelImpl>,
  nodes: Box<dyn VirtualCollectionSelfContained<NodeIdentity, NodeGPU>>,
  std_model: StandardModelGPUResource,
}

impl SceneModelGPUResource {
  pub fn render_scene_model_gles_style(
    &self,
    sm: &SceneModel,
    base: &dyn RenderComponentAny,
    camera: &CameraGPU,
    pass: &mut GPURenderPassCtx,
  ) {
    let sm = self.sm.get(sm.alloc_index().into()).unwrap();
    let node = self.nodes.access_ref(&sm.node.scene_and_node_id()).unwrap();
    match &sm.model {
      ModelEnum::Standard(std) => {
        let (mat, mesh, draw_command) = self.std_model.prepare_render(std.alloc_index().into());
        dispatch_model_draw_with_preferred_binding_frequency(
          base,
          mesh,
          node,
          camera,
          mat,
          draw_command,
          pass,
        )
      }
      ModelEnum::Foreign(_) => todo!(),
    }
  }
}

pub fn dispatch_model_draw_with_preferred_binding_frequency(
  base: &dyn RenderComponentAny,
  mesh: SceneMeshRenderComponent,
  node: &NodeGPU,
  camera: &CameraGPU,
  material: SceneMaterialRenderComponent,
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

pub struct StandardModelGPUResource {
  models: StorageReadView<StandardModel>,
  mat: MaterialsGPUResource,
  mesh: MeshGPUResource,
}

impl StandardModelGPUResource {
  pub fn prepare_render(
    &self,
    m: AllocIdx<StandardModel>,
  ) -> (
    SceneMaterialRenderComponent,
    SceneMeshRenderComponent,
    DrawCommand,
  ) {
    let model = self.models.get(m).unwrap();
    let mat = self.mat.prepare_render(&model.material);
    let mesh = self.mesh.prepare_render(&model.mesh);

    let draw_command = mesh.draw_command(model.group);

    (mat, mesh, draw_command)
  }
}
