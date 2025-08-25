use std::marker::PhantomData;

use rendiation_shader_api::{Shader140Array, ShaderSizedValueNodeType, Std140};

use crate::*;

#[derive(Clone, Debug)]
pub struct UniformBufferDataView<T: Std140> {
  pub gpu: GPUBufferResourceView,
  phantom: PhantomData<T>,
}

impl<T: Std140> PartialEq for UniformBufferDataView<T> {
  fn eq(&self, other: &Self) -> bool {
    self.gpu == other.gpu
  }
}

impl<T: Std140> UniformBufferDataView<T> {
  pub fn create_default(device: &GPUDevice) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &GPUDevice, data: T) -> Self {
    let usage = gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST;

    let init = BufferInit::WithInit(data.as_bytes());
    let desc = GPUBufferDescriptor {
      size: init.size(),
      usage,
    };

    let gpu = GPUBuffer::create(device, None, init, usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, desc, device).create_default_view();

    Self {
      gpu,
      phantom: PhantomData,
    }
  }

  pub fn write_at<D: Pod>(&self, queue: &gpu::Queue, data: &D, offset: u64) {
    queue.write_buffer(&self.gpu.resource.gpu, offset, bytemuck::bytes_of(data));
  }
}

impl<T: Std140> BindableResourceProvider for UniformBufferDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std140> CacheAbleBindingSource for UniformBufferDataView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_binding_build_source()
  }
}
impl<T: Std140> BindableResourceView for UniformBufferDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

/// just short convenient method
pub fn create_uniform<T: Std140>(
  data: T,
  device: impl AsRef<GPUDevice>,
) -> UniformBufferDataView<T> {
  UniformBufferDataView::create(device.as_ref(), data)
}

/// Typed uniform buffer with cpu data cache, which could being diffed when updating to gpu
#[derive(Clone)]
pub struct UniformBufferCachedDataView<T: Std140> {
  gpu: UniformBufferDataView<T>,
  diff: Arc<RwLock<DiffState<T>>>,
}

impl<T: Std140> PartialEq for UniformBufferCachedDataView<T> {
  fn eq(&self, other: &Self) -> bool {
    self.gpu == other.gpu
  }
}

/// just short convenient method
pub fn create_uniform_with_cache<T: Std140>(
  data: T,
  device: impl AsRef<GPUDevice>,
) -> UniformBufferCachedDataView<T> {
  UniformBufferCachedDataView::create(device.as_ref(), data)
}

impl<T: Std140> BindableResourceProvider for UniformBufferCachedDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std140> CacheAbleBindingSource for UniformBufferCachedDataView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_binding_build_source()
  }
}
impl<T: Std140> BindableResourceView for UniformBufferCachedDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

impl<T: Std140> UniformBufferCachedDataView<T> {
  pub fn create_default(device: &GPUDevice) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &GPUDevice, data: T) -> Self {
    Self {
      gpu: UniformBufferDataView::create(device, data),
      diff: Arc::new(RwLock::new(DiffState::new(data))),
    }
  }

  pub fn mutate(&self, f: impl FnOnce(&mut T)) -> &Self {
    let mut state = self.diff.write();
    f(&mut state.data);
    state.changed = true;
    self
  }

  pub fn get(&self) -> T {
    self.diff.read().data
  }

  pub fn set(&self, v: T) {
    let mut state = self.diff.write();
    state.data = v;
    state.changed = true;
  }

  pub fn upload(&self, queue: &gpu::Queue) {
    let mut state = self.diff.write();
    if state.changed {
      let data = state.data;
      queue.write_buffer(&self.gpu.gpu.resource.gpu, 0, bytemuck::cast_slice(&[data]));
      state.changed = false;
      state.last = Some(data);
    }
  }

  pub fn upload_with_diff(&self, queue: &gpu::Queue)
  where
    T: PartialEq,
  {
    let mut state = self.diff.write();
    if state.changed {
      let data = state.data;
      let should_update;

      // if last is none, means we use init value, not need update
      if let Some(last) = state.last {
        should_update = last != data;
        state.last = Some(data);
      } else {
        should_update = true;
      }

      if should_update {
        queue.write_buffer(&self.gpu.gpu.resource.gpu, 0, bytemuck::cast_slice(&[data]))
      }

      state.changed = false;
    }
  }
}

struct DiffState<T> {
  data: T,
  last: Option<T>,
  changed: bool,
}

impl<T> DiffState<T> {
  pub fn new(data: T) -> Self {
    Self {
      data,
      last: None,
      changed: false,
    }
  }
}

pub struct ClampedUniformList<T: Std140, const N: usize> {
  pub source: Vec<T>,
  pub gpu: Option<UniformBufferCachedDataView<Shader140Array<T, N>>>,
}

impl<T: Std140, const N: usize> Default for ClampedUniformList<T, N> {
  fn default() -> Self {
    Self {
      source: Default::default(),
      gpu: Default::default(),
    }
  }
}

impl<T: Std140 + Default, const N: usize> ClampedUniformList<T, N> {
  pub fn reset(&mut self) {
    self.source.clear();
    self.gpu.take();
  }

  pub fn update_gpu(&mut self, gpu: &GPUDevice) -> usize {
    let mut source = vec![T::default(); N];
    for (i, light) in self.source.iter().enumerate() {
      if i >= N {
        break;
      }
      source[i] = *light;
    }
    let source = source.try_into().unwrap();
    let lights_gpu = create_uniform_with_cache(source, gpu);
    self.gpu = lights_gpu.into();
    self.source.len()
  }
}

impl<T, const N: usize> ShaderPassBuilder for ClampedUniformList<T, N>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.gpu.as_ref().unwrap());
  }
}

impl<T: Std140, const N: usize> ShaderHashProvider for ClampedUniformList<T, N> {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
  shader_hash_type_id! {}
}
