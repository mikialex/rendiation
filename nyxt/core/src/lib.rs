use std::{cell::RefCell, rc::Rc};

use rendiation_ral::*;
use rendiation_webgl::WebGL;

pub mod handle;
pub use handle::*;

pub type GFX = WebGL;

pub trait NyxtViewerInnerTrait {
  type ViewerReal;
  fn mutate_inner<T>(real: &Self::ViewerReal, mutator: impl FnOnce(&mut Self) -> T) -> T;
  fn get_inner(real: &Self::ViewerReal) -> Rc<RefCell<Self>>;

  fn get_uniform<T: Copy + 'static>(&self, handle: UniformHandleWrap<T>) -> &T;
  fn get_uniform_mut<T: Copy + 'static>(&self, handle: UniformHandleWrap<T>) -> &mut T;
  fn delete_uniform<T: Copy + 'static>(&self, handle: UniformHandleWrap<T>);

  fn get_bindgroup<T: BindGroupProvider<GFX>>(
    &self,
    handle: BindGroupHandleWrap<T>,
  ) -> &T::Instance;
  fn get_bindgroup_mut<T: BindGroupProvider<GFX>>(
    &self,
    handle: BindGroupHandleWrap<T>,
  ) -> &mut T::Instance;
  fn delete_bindgroup<T: BindGroupProvider<GFX>>(&self, handle: BindGroupHandleWrap<T>);

  fn get_shading<T: ShadingProvider<GFX>>(&self, handle: ShadingHandleWrap<T>) -> &T::Instance;
  fn get_shading_mut<T: ShadingProvider<GFX>>(
    &self,
    handle: ShadingHandleWrap<T>,
  ) -> &mut T::Instance;
  fn delete_shading<T: ShadingProvider<GFX>>(&self, handle: ShadingHandleWrap<T>);
}

pub trait NyxtViewerHandle<V: NyxtViewerInnerTrait>: Copy {
  type Item;

  fn get(self, inner: &V) -> &Self::Item;
  fn free(self, inner: &mut V);
}

pub trait NyxtViewerMutableHandle<V: NyxtViewerInnerTrait>: NyxtViewerHandle<V> {
  fn get_mut(self, inner: &mut V) -> &mut Self::Item;
}

pub trait NyxtShadingWrapped<V: NyxtViewerInnerTrait>: ShadingProvider<GFX> + Sized {
  type Wrapper;

  fn to_nyxt_wrapper(viewer: &Rc<RefCell<V>>, handle: ShadingHandle<GFX, Self>) -> Self::Wrapper;
}

pub struct ShadingHandleWrap<T: ShadingProvider<GFX>>(pub ShadingHandle<GFX, T>);
impl<T: ShadingProvider<GFX>> Copy for ShadingHandleWrap<T> {}
impl<T: ShadingProvider<GFX>> Clone for ShadingHandleWrap<T> {
  fn clone(&self) -> Self {
    ShadingHandleWrap(self.0.clone())
  }
}

impl<V: NyxtViewerInnerTrait, T: ShadingProvider<GFX>> NyxtViewerHandle<V>
  for ShadingHandleWrap<T>
{
  type Item = <T as rendiation_ral::ShadingProvider<GFX>>::Instance;

  fn get(self, inner: &V) -> &Self::Item {
    inner.get_shading(self)
  }
  fn free(self, inner: &mut V) {
    inner.delete_shading(self)
  }
}
impl<V: NyxtViewerInnerTrait, T: ShadingProvider<GFX>> NyxtViewerMutableHandle<V>
  for ShadingHandleWrap<T>
{
  fn get_mut(self, inner: &mut V) -> &mut Self::Item {
    inner.get_shading_mut(self)
  }
}

pub trait NyxtBindGroupWrapped<V: NyxtViewerInnerTrait>: BindGroupProvider<GFX> + Sized {
  type Wrapper;

  fn to_nyxt_wrapper(viewer: &Rc<RefCell<V>>, handle: BindGroupHandle<GFX, Self>) -> Self::Wrapper;
}

pub struct BindGroupHandleWrap<T: BindGroupProvider<GFX>>(pub BindGroupHandle<GFX, T>);
impl<T: BindGroupProvider<GFX>> Copy for BindGroupHandleWrap<T> {}
impl<T: BindGroupProvider<GFX>> Clone for BindGroupHandleWrap<T> {
  fn clone(&self) -> Self {
    BindGroupHandleWrap(self.0.clone())
  }
}

impl<V: NyxtViewerInnerTrait, T: BindGroupProvider<GFX>> NyxtViewerHandle<V>
  for BindGroupHandleWrap<T>
{
  type Item = <T as rendiation_ral::BindGroupProvider<GFX>>::Instance;

  fn get(self, inner: &V) -> &Self::Item {
    inner.get_bindgroup(self)
  }
  fn free(self, inner: &mut V) {
    inner.delete_bindgroup(self)
  }
}
impl<V: NyxtViewerInnerTrait, T: BindGroupProvider<GFX>> NyxtViewerMutableHandle<V>
  for BindGroupHandleWrap<T>
{
  fn get_mut(self, inner: &mut V) -> &mut Self::Item {
    inner.get_bindgroup_mut(self)
  }
}

pub trait NyxtUBOWrapped<V: NyxtViewerInnerTrait>: Sized {
  type Wrapper;

  fn to_nyxt_wrapper(viewer: &Rc<RefCell<V>>, handle: UniformHandle<GFX, Self>) -> Self::Wrapper;
}

#[derive(Copy, Clone)]
pub struct UniformHandleWrap<T>(pub UniformHandle<GFX, T>);

impl<V: NyxtViewerInnerTrait, T: Copy + 'static> NyxtViewerHandle<V> for UniformHandleWrap<T> {
  type Item = T;

  fn get(self, inner: &V) -> &Self::Item {
    inner.get_uniform(self)
  }
  fn free(self, inner: &mut V) {
    inner.delete_uniform(self)
  }
}
impl<V: NyxtViewerInnerTrait, T: Copy + 'static> NyxtViewerMutableHandle<V>
  for UniformHandleWrap<T>
{
  fn get_mut(self, inner: &mut V) -> &mut Self::Item {
    inner.get_uniform_mut(self)
  }
}
