use crate::*;

pub trait CacheAbleBindingSource {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource;
}

impl<T> CacheAbleBindingSource for ResourceViewRc<T>
where
  T: Resource,
  Self: BindableResourceProvider,
{
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: self.get_bindable(),
      view_id: self.guid,
    }
  }
}

pub struct CacheAbleBindingBuildSource {
  pub(crate) source: BindingResourceOwned,
  pub(crate) view_id: usize,
}

impl CacheAbleBindingBuildSource {
  pub fn build_bindgroup(
    sources: &[Self],
    device: &GPUDevice,
    layout: &gpu::BindGroupLayout,
  ) -> gpu::BindGroup {
    let entries_prepare: Vec<_> = sources.iter().map(|s| s.source.prepare_ref()).collect();
    let entries: Vec<_> = entries_prepare
      .iter()
      .enumerate()
      .map(|(i, s)| gpu::BindGroupEntry {
        binding: i as u32,
        resource: s.as_binding_ref(),
      })
      .collect();

    device.create_bind_group(&gpu::BindGroupDescriptor {
      label: None,
      layout,
      entries: &entries,
    })
  }
}
