use rendiation_scene_rendering_gpu_gles::{GLESNodeRenderImpl, NodeGPUUniform, NodeUniform};

use crate::*;

pub fn use_view_dependent_transform_gles_gpu(
  cx: &mut QueryGPUHookCx,
  changes: UseResult<BoxedDynDualQuery<ViewSceneModelKey, Mat4<f64>>>,
  internal_impl: Option<Box<dyn GLESNodeRenderImpl>>,
  control: CurrentViewControl,
) -> Option<Box<dyn GLESNodeRenderImpl>> {
  let changes = changes.use_assure_result(cx);

  let overrides = cx.use_shared_hash_map::<ViewSceneModelKey, UniformBufferDataView<NodeUniform>>(
    "OverrideNodeIndirectGPU",
  );

  if let GPUQueryHookStage::CreateRender { .. } = &mut cx.stage {
    let query = changes.expect_resolve_stage();

    let mut overrides_ = overrides.write();
    let overrides = &mut overrides_;
    let changes = query.delta.into_change();

    for vk in changes.iter_removed() {
      overrides.remove(&vk);
    }

    for (vk, mat) in changes.iter_update_or_insert() {
      let node = NodeUniform::from_world_mat(mat);
      let node = UniformBufferDataView::create(&cx.gpu.device, node);
      overrides.insert(vk, node);
    }
  }

  internal_impl.map(|internal| {
    Box::new(OverrideNodeGlesGPU {
      internal,
      overrides: overrides.make_read_holder(),
      current_view: control,
    }) as Box<_>
  })
}

struct OverrideNodeGlesGPU {
  internal: Box<dyn GLESNodeRenderImpl>,
  overrides:
    LockReadGuardHolder<FastHashMap<ViewSceneModelKey, UniformBufferDataView<NodeUniform>>>,
  current_view: CurrentViewControl,
}

impl GLESNodeRenderImpl for OverrideNodeGlesGPU {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneNodeEntity>,
    sm: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    if let Some(current_view) = self.current_view.get() {
      if let Some(ubo) = self.overrides.get(&(current_view, sm.into_raw())) {
        Some(Box::new(NodeGPUUniform { ubo }))
      } else {
        self.internal.make_component(idx, sm)
      }
    } else {
      self.internal.make_component(idx, sm)
    }
  }
}
