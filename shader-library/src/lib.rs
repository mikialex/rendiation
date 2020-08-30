pub mod builtin;
pub mod fog;
pub mod sph;
pub mod tone_mapping;
pub mod transform;

pub use rendiation_math::*;
pub use rendiation_shadergraph::*;
pub use rendiation_shadergraph_derives::*;

// use rendiation_ral::*;
// use rendiation_webgpu::WGPURenderer;
// use transform::MVPTransformation;
// pub struct BlockShadingParamGroup;
// pub struct BlockShadingParamGroupInstance<WGPURenderer> {
//   pub mvp: UniformHandle<WGPURenderer, MVPTransformation>,
// }

// impl rendiation_ral::BindGroupProvider<WGPURenderer>
//   for BlockShadingParamGroupInstance<WGPURenderer>
// {
//   fn create_bindgroup(
//     &self,
//     renderer: &<WGPURenderer as rendiation_ral::RALBackend>::Renderer,
//     resources: &rendiation_ral::ShaderBindableResourceManager<WGPURenderer>,
//   ) -> <WGPURenderer as rendiation_ral::RALBackend>::BindGroup {
//     renderer.register_bindgroup::<Self>();

//     let mvp = <MVPTransformation as rendiation_ral::RALBindgroupItem<WGPURenderer>>::get_item(
//       self.mvp, resources,
//     );

//     rendiation_webgpu::BindGroupBuilder::new()
//       // #(#wgpu_create_bindgroup_create)*
//       .push(<MVPTransformation as rendiation_shadergraph::WGPUBindgroupItem>::to_binding(mvp))
//       .build(
//         &renderer.device,
//         renderer
//           .bindgroup_layout_cache
//           .borrow()
//           .get(&std::any::TypeId::of::<BlockShadingParamGroup>())
//           .unwrap(),
//       )
//   }

//   fn apply(
//     &self,
//     render_pass: &mut <WGPURenderer as rendiation_ral::RALBackend>::RenderPass,
//     gpu_bindgroup: &<WGPURenderer as rendiation_ral::RALBackend>::BindGroup,
//   ) {
//   }
// }
