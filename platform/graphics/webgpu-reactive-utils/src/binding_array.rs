use crate::*;

pub struct BindingArrayMaintainer<K, V> {
  upstream: Box<dyn ReactiveCollection<K, V>>,
  array: Option<BindingResourceArray<V>>,
  default_instance: V,
  max_length: u32,
}

impl<K, V> BindingArrayMaintainer<K, V> {
  /// max_length is used to limit the length of the binding array. should be less than platform
  /// limitation with consideration of the resource usage outside of the binding array.
  ///
  /// if max_length is small, bindless is useless, if max_length is big, the bindless array update
  /// will be costly.
  ///
  /// todo, provide another internal resizable binding length control
  pub fn new(upstream: Box<dyn ReactiveCollection<K, V>>, default: V, max_length: u32) -> Self {
    Self {
      upstream,
      array: Default::default(),
      default_instance: default,
      max_length,
    }
  }
}

impl<K, V> BindingArrayMaintainer<K, V>
where
  K: CKey + LinearIdentified,
  V: CValue,
{
  pub fn poll_update(&mut self, cx: &mut Context) -> BindingResourceArray<V> {
    // detail change info is useless here because the binding array update can not be preformed
    // incrementally. but we still keep the form of full reactive collection to do optimization in
    // future if the wgpu provide the binding array incremental update method.
    if self.upstream.poll_changes(cx).is_ready() {
      let full_view = self.upstream.access();
      let mut new_source = vec![self.default_instance.clone(); self.max_length as usize];
      for (k, v) in full_view.iter_key_value() {
        new_source[k.alloc_index() as usize] = v.clone();
      }
      self.array = BindingResourceArray::<V>::new(Arc::new(new_source), self.max_length).into();
    }
    self.array.clone().unwrap()
  }
}
