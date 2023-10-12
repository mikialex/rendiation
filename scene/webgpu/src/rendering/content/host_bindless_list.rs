use crate::*;

pub enum ModelMaybeBindlessDraw {
  Origin(SceneModel),
  Bindless((BindlessMeshDispatcher, SceneModel)),
}

pub struct MaybeBindlessMeshRenderList {
  origin: RenderList,
  opaque_override: Vec<ModelMaybeBindlessDraw>,
  enable_bindless: bool,
}

impl MaybeBindlessMeshRenderList {
  // this is not good, should be optimized heavily
  pub fn from_list(list: RenderList, scene: &SceneRenderResourceGroup) -> Self {
    if let Some(system) = scene.resources.bindable_ctx.bindless_mesh.as_ref() {
      let meshes_gpu = &scene.resources.model_ctx.meshes.read().unwrap();

      let mut bindless_grouper = FastHashMap::default();

      let mut opaque_override = Vec::with_capacity(list.opaque.len());

      for (index, (m, _)) in list.opaque.iter().enumerate() {
        //   if model.read().node.get_world
        if let ModelEnum::Standard(model) = &m.read().model {
          let model = model.read();
          let mesh_id = model.mesh.guid().unwrap();
          if let Some(mesh_gpu) = meshes_gpu.get(&mesh_id) {
            if let Some(mesh_handle) = mesh_gpu.get_bindless() {
              let collected = bindless_grouper
                .entry(model.material.guid())
                .or_insert_with(|| (Vec::default(), index));
              collected.0.push(mesh_handle);
              continue;
            }
          }
        }
        opaque_override.push(ModelMaybeBindlessDraw::Origin(m.clone()));
      }

      for (mesh_handles, any_model) in bindless_grouper.values() {
        let dispatcher = system
          .create_host_draw_dispatcher(mesh_handles.iter().copied(), &scene.resources.gpu.device);
        opaque_override.push(ModelMaybeBindlessDraw::Bindless((
          dispatcher,
          list.opaque[*any_model].0.clone(),
        )));
      }
      //
      Self {
        origin: list,
        opaque_override,
        enable_bindless: true,
      }
    } else {
      Self {
        origin: list,
        opaque_override: Default::default(),
        enable_bindless: false,
      }
    }
  }

  pub fn setup_pass(
    &self,
    gpu_pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    resource: &SceneRenderResourceGroup,
  ) {
    let resource_view = ModelGPURenderResourceView::new(resource);
    let camera_gpu = resource_view.cameras.get_camera_gpu(camera).unwrap();

    for item in &self.opaque_override {
      match item {
        ModelMaybeBindlessDraw::Origin(model) => {
          scene_model_setup_pass_core(
            gpu_pass,
            model.guid(),
            camera_gpu,
            &resource_view,
            dispatcher,
          );
        }
        ModelMaybeBindlessDraw::Bindless((system, model)) => {
          scene_model_setup_pass_core(
            gpu_pass,
            model.guid(),
            camera_gpu,
            &resource_view,
            &BindlessMeshProvider {
              base: &dispatcher,
              system,
            },
          );
          gpu_pass.draw_by_command(system.draw_command());
        }
      };
    }

    self
      .origin
      .setup_pass(gpu_pass, dispatcher, camera, resource, self.enable_bindless)
  }
}
