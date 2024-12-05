use crate::*;

#[derive(Clone)]
pub struct GPUInstance {
  instance: Arc<GPUInstanceInner>,
}

impl Deref for GPUInstance {
  type Target = gpu::Instance;

  fn deref(&self) -> &Self::Target {
    &self.instance.instance
  }
}

impl GPUInstance {
  pub fn new(instance: gpu::Instance) -> Self {
    let instance = Arc::new(instance);
    let instance_clone = instance.clone();

    let dropped = Arc::new(AtomicBool::new(false));
    let dropped_clone = dropped.clone();
    // wasm can not spawn thread, add the gpu will be automatically polled by runtime(browser)
    #[cfg(not(target_family = "wasm"))]
    std::thread::spawn(move || loop {
      if dropped_clone.load(Ordering::Relaxed) {
        break;
      }
      std::thread::sleep(std::time::Duration::from_millis(200));
      instance_clone.poll_all(false);
    });

    Self {
      instance: Arc::new(GPUInstanceInner {
        instance,
        is_dropped: dropped,
      }),
    }
  }
}

pub struct GPUInstanceInner {
  instance: Arc<gpu::Instance>,
  is_dropped: Arc<AtomicBool>,
}

impl Drop for GPUInstanceInner {
  fn drop(&mut self) {
    self.is_dropped.store(true, Ordering::Relaxed);
  }
}
