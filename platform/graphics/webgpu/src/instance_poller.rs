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

#[cfg(not(target_family = "wasm"))]
impl GPUInstance {
  pub fn enable_spin_polling(&self) {
    self.set_polling_frequency(0);
  }
  // if set to 0, the polling will be spinning
  pub fn set_polling_frequency(&self, ms: u32) {
    self.instance.polling_frequency.store(ms, Ordering::Relaxed);
  }
  pub fn get_polling_frequency(&self) -> u32 {
    self.instance.polling_frequency.load(Ordering::Relaxed)
  }
}

impl GPUInstance {
  // wasm can not spawn thread, add the gpu will be automatically polled by runtime(browser)
  #[cfg(target_family = "wasm")]
  pub fn new(instance: gpu::Instance) -> Self {
    Self {
      instance: Arc::new(GPUInstanceInner {
        instance: Arc::new(instance),
      }),
    }
  }

  #[cfg(not(target_family = "wasm"))]
  pub fn new(instance: gpu::Instance) -> Self {
    let instance = Arc::new(instance);
    let polling_frequency = Arc::new(AtomicU32::new(16));
    let dropped = Arc::new(AtomicBool::new(false));

    {
      let instance_clone = instance.clone();
      let dropped_clone = dropped.clone();
      let polling_frequency_clone = polling_frequency.clone();

      std::thread::spawn(move || loop {
        if dropped_clone.load(Ordering::Relaxed) {
          break;
        }
        let polling_frequency = polling_frequency_clone.load(Ordering::Relaxed);
        if polling_frequency == 0 {
          instance_clone.poll_all(false);
        } else {
          std::thread::sleep(std::time::Duration::from_millis(polling_frequency as u64));
          instance_clone.poll_all(false);
        }
      });
    }

    Self {
      instance: Arc::new(GPUInstanceInner {
        instance,
        is_dropped: dropped,
        polling_frequency,
      }),
    }
  }
}

pub struct GPUInstanceInner {
  instance: Arc<gpu::Instance>,
  #[cfg(not(target_family = "wasm"))]
  is_dropped: Arc<AtomicBool>,
  #[cfg(not(target_family = "wasm"))]
  polling_frequency: Arc<AtomicU32>,
}

impl Drop for GPUInstanceInner {
  fn drop(&mut self) {
    #[cfg(not(target_family = "wasm"))]
    self.is_dropped.store(true, Ordering::Relaxed);
  }
}
