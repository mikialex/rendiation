use core::ops::Deref;
use core::ops::DerefMut;
use rendiation::*;

pub struct Watch<T> {
  item: T,
  version: usize,
}

impl<T> Watch<T> {
  pub fn new(item: T) -> Self {
    Watch { item, version: 0 }
  }
  pub fn mutate(&mut self) -> &mut T {
    self.version += 1;
    &mut self.item
  }
  pub fn get_version(&self) -> usize{
    self.version
  }
}

impl<T> Deref for Watch<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &self.item
  }
}
impl<T> DerefMut for Watch<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.mutate()
  }
}

pub struct GPUPair<T, G: GPUItem<T>>{
  watched: Watch<T>,
  gpu: G,
  synced_version: usize,
}

impl<T, G: GPUItem<T>> GPUPair<T, G>{
  pub fn new<R: Renderer>(item: T, renderer: &mut WGPURenderer<R>)-> Self{
    let gpu = G::create_gpu(&item, renderer);
    let watched = Watch::new(item);
    GPUPair{
      watched,
      gpu,
      synced_version: 0,
    }    
  }

  pub fn get_update_gpu<R: Renderer>(&mut self, renderer: &mut WGPURenderer<R>) -> &G{
    if self.watched.get_version() != self.synced_version {
      self.gpu.update_gpu(&self.watched, renderer);
    }
    &self.gpu
  }
}

impl<T, G: GPUItem<T>> Deref for GPUPair<T, G> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &self.watched
  }
}
impl<T, G: GPUItem<T>> DerefMut for GPUPair<T, G> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.watched
  }
}

pub trait GPUItem<T>{
  fn create_gpu<R: Renderer>(item: &T, renderer: &mut WGPURenderer<R>) -> Self;
  fn update_gpu<R: Renderer>(&mut self, item: &T, renderer: &mut WGPURenderer<R>);
}