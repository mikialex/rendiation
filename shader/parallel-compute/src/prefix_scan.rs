use crate::*;

// struct WorkGroupPrefixSum<V> {
//   upstream: Box<dyn DeviceParallelCompute<Node<V>>>,
//   workgroup_usage: Node<ShaderWorkGroupPtr<[V; 128]>>,
// }

// impl<K, V> GPUParallelComputation<K, Node<V>> for WorkGroupPrefixSum<K, V>
// where
//   V: ShaderNodeType + DeviceMonoid,
// {
//   fn thread_logic(&self , key: K) -> Node<V> {
//     let input = self.upstream.thread_logic(key);
//     let shared = self.workgroup_usage;

//     let local_id = local_invocation_id().x();

//     let value = input.make_local_var();

//     shared.index(local_id).store(value.load());

//     128.ilog2().into_shader_iter().for_each(|i, _| {
//       workgroup_barrier();

//       if_by(local_id.greater_equal_than(val(1) << i), || {
//         let a = value.load();
//         let b = shared.index(local_id - (val(1) << i)).load();
//         let combined = V::combine(a, b);
//         value.store(combined)
//       });

//       workgroup_barrier();
//       shared.index(local_id).store(value.load())
//     });

//     value.load()
//   }
// }
