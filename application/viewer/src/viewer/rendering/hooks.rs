use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub struct Viewer3dRenderingCx<'a> {
  memory: usize,
  dyn_cx: &'a DynCx,
  pub stage: Viewer3dRenderingCxStage<'a>,
  gpu: &'a GPU,
}

impl<'a> QueryGPUHookCx for Viewer3dRenderingCx<'a> {
  fn use_multi_updater<T>(
    &mut self,
    f: impl FnOnce() -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    todo!()
  }

  fn use_uniform_buffers<K, V: Std140>(
    &mut self,
    source: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> Option<LockReadGuardHolder<UniformUpdateContainer<K, V>>> {
    todo!()
  }

  fn use_storage_buffer<V: Std430>(
    &mut self,
    source: impl FnOnce(&GPU) -> ReactiveStorageBufferContainer<V>,
  ) -> Option<StorageBufferReadonlyDataView<[V]>> {
    todo!()
  }

  fn use_multi_updater_ref<T>(
    &mut self,
    f: impl FnOnce(&GPU) -> MultiUpdateContainer<T>,
  ) -> (&mut Self, Option<&T>) {
    todo!()
  }

  fn use_uniform_buffers_ref<K, V: Std140>(
    &mut self,
    source: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> (
    &mut Self,
    Option<&fast_hash_collection::FastHashMap<K, UniformBufferDataView<V>>>,
  ) {
    todo!()
  }

  fn use_global_multi_reactive_query<D: ForeignKeySemantic>(
    &mut self,
  ) -> Option<
    Box<dyn DynMultiQuery<Key = EntityHandle<D::ForeignEntity>, Value = EntityHandle<D::Entity>>>,
  > {
    todo!()
  }

  fn use_reactive_query<K, V, Q: ReactiveQuery<Key = K, Value = V>>(
    &mut self,
    source: impl FnOnce() -> Q,
  ) -> Option<Box<dyn DynQuery<Key = K, Value = V>>> {
    todo!()
  }

  fn use_val_refed_reactive_query<K, V, Q: ReactiveValueRefQuery<Key = K, Value = V>>(
    &mut self,
    source: impl FnOnce(&GPU) -> Q,
  ) -> Option<Box<dyn DynValueRefQuery<Key = K, Value = V>>> {
    todo!()
  }

  fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    todo!()
  }
}

impl<'a> Viewer3dRenderingCx<'a> {
  pub fn use_plain_state<T>(&mut self) -> (&mut Self, &mut T) {
    todo!()
  }
  pub fn use_plain_state_init<T>(&mut self, init: &T) -> (&mut Self, &mut T) {
    todo!()
  }
  pub fn use_plain_state_init_by<T>(&mut self, init: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    todo!()
  }

  pub fn use_gpu_state<T>(&mut self, init: impl FnOnce(&GPU) -> T) -> (&mut Self, &mut T) {
    todo!()
  }

  pub fn on_render<R>(
    &mut self,
    f: impl FnOnce(&mut Viewer3dRenderingCxRenderStage) -> R,
  ) -> Option<R> {
    if let Viewer3dRenderingCxStage::Render(render) = &mut self.stage {
      return Some(f(render));
    } else {
      None
    }
  }

  pub fn on_gui<R>(&mut self, f: impl FnOnce(&'a mut egui::Ui) -> R) -> Option<R> {
    None
  }
}

pub enum Viewer3dRenderingCxStage<'a> {
  Init {},
  Uninit {},
  Render(Viewer3dRenderingCxRenderStage<'a>),
  Gui { context: &'a mut egui::Ui },
}

pub struct Viewer3dRenderingCxRenderStage<'a> {
  pub target: RenderTargetView,
  pub content: &'a Viewer3dSceneCtx,
  pub frame: FrameCtx<'a>,
  pub dyn_cx: &'a mut DynCx,
}
