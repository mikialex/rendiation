use std::hash::Hasher;

use fast_hash_collection::FastHashMap;

use crate::*;

pub fn use_indirect_renderer(
  cx: &mut impl QueryGPUHookCx,
  reversed_depth: bool,
  materials: Option<Box<dyn IndirectModelMaterialRenderImpl>>,
  mesh: Option<MeshGPUBindlessImpl>,
  texture_system: Option<GPUTextureBindingSystem>,
) -> Option<IndirectSceneRenderer> {
  let mesh = mesh.map(|v| Box::new(v) as Box<dyn IndirectModelShapeRenderImpl>);

  let std_model = use_std_model_renderer(cx, materials, mesh);

  let scene_model = use_indirect_scene_model(cx, std_model.map(|v| Box::new(v) as Box<_>));

  cx.when_render(|| IndirectSceneRenderer {
    texture_system: texture_system.unwrap(),
    renderer: scene_model.map(|v| Box::new(v) as Box<_>).unwrap(),
    reversed_depth,
  })
}

pub struct IndirectSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  renderer: Box<dyn IndirectBatchSceneModelRenderer>,
  reversed_depth: bool,
}

impl SceneModelRenderer for IndirectSceneRenderer {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    self.renderer.render_scene_model(idx, camera, pass, cx, tex)
  }
}

impl IndirectSceneRenderer {
  fn create_batch_from_iter(
    &self,
    iter: impl Iterator<Item = EntityHandle<SceneModelEntity>>,
  ) -> DeviceSceneModelRenderBatch {
    let mut classifier = FastHashMap::default();

    for sm in iter {
      let mut hasher = PipelineHasher::default();
      self
        .renderer
        .hash_shader_group_key_with_self_type_info(sm, &mut hasher)
        .expect("unable to find indirect group key for scene_model");
      let shader_hash = hasher.finish();
      let list = classifier.entry(shader_hash).or_insert_with(Vec::new);
      list.push(sm);
    }

    let sub_batches = classifier
      .drain()
      .map(|(_, list)| {
        let scene_models: Vec<_> = list.iter().map(|sm| sm.alloc_index()).collect();
        let scene_models = Box::new(scene_models);

        DeviceSceneModelRenderSubBatch {
          scene_models,
          impl_select_id: *list.first().unwrap(),
        }
      })
      .collect();

    DeviceSceneModelRenderBatch {
      sub_batches,
      stash_culler: None,
    }
  }
}

impl SceneRenderer for IndirectSceneRenderer {
  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let batch = match batch {
      SceneModelRenderBatch::Device(batch) => batch,
      SceneModelRenderBatch::Host(batch) => self.create_batch_from_iter(batch.iter_scene_models()),
    };

    let batch = ctx.access_parallel_compute(|cx| batch.flush_culler_into_new(cx));

    let content: Vec<_> = batch
      .sub_batches
      .iter()
      .map(|batch| {
        let provider = self.renderer.generate_indirect_draw_provider(batch, ctx);
        (provider, batch.impl_select_id)
      })
      .collect();

    Box::new(IndirectScenePassContent {
      renderer: self,
      content,
      pass,
      camera,
      reversed_depth: self.reversed_depth,
    })
  }
}

struct IndirectScenePassContent<'a> {
  renderer: &'a IndirectSceneRenderer,
  content: Vec<(
    Box<dyn IndirectDrawProvider>,
    EntityHandle<SceneModelEntity>,
  )>,

  pass: &'a dyn RenderComponent,
  camera: &'a dyn RenderComponent,
  reversed_depth: bool,
}

impl PassContent for IndirectScenePassContent<'_> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    let base = default_dispatcher(cx, self.reversed_depth).disable_auto_write();
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    for (content, any_scene_model) in &self.content {
      self.renderer.renderer.render_indirect_batch_models(
        content.as_ref(),
        *any_scene_model,
        &self.camera,
        &self.renderer.texture_system,
        &p,
        &mut cx.ctx,
      );
    }
  }
}
