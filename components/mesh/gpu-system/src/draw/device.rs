// impl GPUBindlessMeshSystem {  /// user could use this in their compute shader to generate the
// buffer we want   pub fn prepare_generate_draw_command_in_device(
//     &self,
//     cx: &ComputeCx,
//   ) -> BindlessMeshDrawGeneratorInDevice {
//    todo!()
//   }
// }

// pub struct BindlessMeshDrawGeneratorInDevice {
//   node: ReadOnlyStorageNode<[DrawMetaData]>,
// }

// impl BindlessMeshDrawGeneratorInDevice {
//   pub fn generate_draw_command(
//     &self,
//     mesh_handle: Node<u32>,
//   ) -> (ENode<DrawIndirect>, ENode<DrawVertexIndirectInfo>) {
//     let meta = self.node.index(mesh_handle).load().expand();
//     let draw = ENode::<DrawIndirect> {
//         vertex_count: meta.count,
//         instance_count: val(1),
//         base_vertex: meta.start,
//         base_instance: todo!(),
//     }
//     todo!()
//   }
// }
