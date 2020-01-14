use core::ops::Deref;
use core::ops::DerefMut;
use rendiation::*;

use std::sync::atomic::{AtomicUsize, Ordering};
static GLOBAL_TEXTURE_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Watch<T> {
  item: T,
  version: usize,
  guid: usize,
}

impl<T> Watch<T> {
  pub fn new(item: T) -> Self {
    Watch {
      item,
      version: 0,
      guid: GLOBAL_TEXTURE_ID.fetch_add(1, Ordering::SeqCst),
    }
  }
  pub fn mutate(&mut self) -> &mut T {
    self.version += 1;
    &mut self.item
  }
  pub fn get_version(&self) -> usize {
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

pub struct GPUPair<T, G: GPUItem<T>> {
  watched: Watch<T>,
  gpu: G,
  synced_version: usize,
}

impl<T, G: GPUItem<T>> GPUPair<T, G> {
  pub fn new(item: T, renderer: &mut WGPURenderer) -> Self {
    let gpu = G::create_gpu(&item, renderer);
    let watched = Watch::new(item);
    GPUPair {
      watched,
      gpu,
      synced_version: 0,
    }
  }

  pub fn get_update_gpu(&mut self, renderer: &mut WGPURenderer) -> &G {
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

pub trait GPUItem<T> {
  fn create_gpu(item: &T, renderer: &mut WGPURenderer) -> Self;
  fn update_gpu(&mut self, item: &T, renderer: &mut WGPURenderer);
}
