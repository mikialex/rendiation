use crate::*;

pub struct BindingArrayMaintainer<V> {
  array: Option<BindingResourceArray<V>>,
  default_instance: V,
  max_length: u32,
}

impl<V: Clone> BindingArrayMaintainer<V> {
  /// max_length is used to limit the length of the binding array. should be less than platform
  /// limitation with consideration of the resource usage outside of the binding array.
  ///
  /// if max_length is small, bindless is useless, if max_length is big, the bindless array update
  /// will be costly.
  ///
  /// todo, provide another internal resizable binding length control
  pub fn new(default: V, max_length: u32) -> Self {
    Self {
      array: Default::default(),
      default_instance: default,
      max_length,
    }
  }

  pub fn get_gpu(&self) -> BindingResourceArray<V> {
    self.array.clone().unwrap()
  }

  // detail change info is useless here because the binding array update can not be preformed
  // incrementally. but we still keep the form of full reactive query to do optimization in
  // future if the wgpu provide the binding array incremental update method.
  pub fn update(&mut self, view: SharedHashMapRead<u32, V>, gpu: &GPU) {
    let mut new_source = vec![self.default_instance.clone(); self.max_length as usize];
    for (k, v) in view.iter() {
      new_source[k.alloc_index() as usize] = v.clone();
    }
    self.array =
      BindingResourceArray::<V>::new(Arc::new(new_source), self.max_length, &gpu.device).into();
  }
}
