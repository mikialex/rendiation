use std::{
  any::{Any, TypeId},
  task::Context,
};

use fast_hash_collection::FastHashMap;

use crate::*;

pub struct GLESStyleGPUResourceSystem {
  pub texture2ds: Box<dyn ReactiveCollection<AllocIdx<SceneTexture2dEntity>, GPU2DTextureView>>,
  pub texture_cubes: CubeMapUpdateContainer<SceneTextureCubeEntity>,
  pub samplers: Box<dyn ReactiveCollection<AllocIdx<SceneSamplerEntity>, GPUSamplerView>>,

  pub attribute_mesh_index_buffers:
    Box<dyn ReactiveCollection<AllocIdx<AttributeMeshEntity>, GPUBufferResourceView>>,
  pub attribute_mesh_vertex_buffers:
    Box<dyn ReactiveCollection<AllocIdx<AttributeMeshVertexBufferRelation>, GPUBufferResourceView>>,

  pub flat_material: FlatMaterialUniforms,
  pub mr_material: PbrMRMaterialUniforms,

  pub nodes: SceneNodeUniforms,
  pub cameras: CameraUniforms,

  pub directional_lights: UniformArrayUpdateContainer<DirectionalLightUniform>,
  pub points_lights: UniformArrayUpdateContainer<PointLightUniform>,
  pub spot_lights: UniformArrayUpdateContainer<SpotLightUniform>,
  pub foreign_support: FastHashMap<TypeId, Box<dyn Any>>,
}

impl GLESStyleGPUResourceSystem {
  pub fn new(cx: &GPUResourceCtx) -> Self {
    Self {
      texture2ds: gpu_texture_2ds(cx).into_boxed(),
      texture_cubes: gpu_texture_cubes(cx),
      samplers: sampler_gpus(cx).into_boxed(),
      directional_lights: directional_uniform_array(cx),
      points_lights: point_uniform_array(cx),
      spot_lights: spot_uniform_array(cx),
      foreign_support: Default::default(),
      nodes: node_gpus(cx),
      cameras: todo!(),
      attribute_mesh_index_buffers: attribute_mesh_index_buffers(cx).into_boxed(),
      attribute_mesh_vertex_buffers: attribute_mesh_vertex_buffer_views(cx).into_boxed(),
      flat_material: flat_material_uniforms(cx),
      mr_material: pbr_mr_material_uniforms(cx),
    }
  }

  pub fn poll_updates(&self, cx: &mut Context) -> GLESStyleGPUResourceSystemAccessView {
    let _ = self.texture2ds.poll_changes(cx);
    let _ = self.samplers.poll_changes(cx);

    GLESStyleGPUResourceSystemAccessView {
      texture2ds: self.texture2ds.access(),
      resource: self,
      foreign_support: Default::default(),
    }
  }
}

pub struct GLESStyleGPUResourceSystemAccessView<'a> {
  pub texture2ds: Box<dyn VirtualCollection<AllocIdx<SceneTexture2dEntity>, GPU2DTextureView>>,
  pub resource: &'a GLESStyleGPUResourceSystem,
  pub foreign_support: FastHashMap<TypeId, Box<dyn Any>>,
}

struct GLESStyleDrawSceneContents<'a> {
  resource: GLESStyleGPUResourceSystemAccessView<'a>,
  render_impl: &'a dyn GLESStyleDrawModelMethodProvider,
  scene_models: Vec<AllocIdx<SceneModelEntity>>,
}

pub trait GLESStyleRenderComponentProvider {
  fn make_component(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    res: GLESStyleGPUResourceSystemAccessView,
  ) -> Option<Box<dyn RenderComponent>>;
}

pub trait GLESStyleDrawModelMethodProvider {
  fn shape_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    res: GLESStyleGPUResourceSystemAccessView,
  ) -> Option<Box<dyn RenderComponent>>;
  fn material_renderable(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    res: GLESStyleGPUResourceSystemAccessView,
  ) -> Option<Box<dyn RenderComponent>>;
  fn draw_command(
    &self,
    idx: AllocIdx<SceneModelEntity>,
    res: GLESStyleGPUResourceSystemAccessView,
  ) -> Option<DrawCommand>;
}

impl<'a> PassContent for GLESStyleDrawSceneContents<'a> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    todo!()
  }
}

// pub trait ReactiveLightCollectionCompute:
//   AsRef<dyn LightCollectionCompute> + Stream<Item = usize> + Unpin
// {
// }
// impl<T> ReactiveLightCollectionCompute for T where
//   T: AsRef<dyn LightCollectionCompute> + Stream<Item = usize> + Unpin
// {
// }

// /// contains gpu data that support forward rendering
// ///
// /// all uniform is update once in a frame. for convenience.
// #[pin_project::pin_project]
// pub struct ForwardLightingSystem {
//   gpu: ResourceGPUCtx,
//   /// note, the correctness now actually rely on the hashmap in stream map provide stable iter in
//   /// stable order. currently, as long as we not insert new collection in runtime, it holds.
//   pub each_light_type: StreamMap<TypeId, Box<dyn ReactiveLightCollectionCompute>>,
// }

// impl<'a> ShaderPassBuilder for ForwardSceneLightingDispatcher<'a> {
//   fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
//     self.base.setup_pass(ctx);
//   }
//   fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
//     self.shadows.setup_pass(ctx);

//     ctx.binding.bind(&self.lights.lengths);
//     for lid in &self.lights.lights_insert_order {
//       let lights = self.lights.lights_collections.get(lid).unwrap();
//       lights.as_ref().as_ref().setup_pass(ctx)
//     }
//     self.lighting.tonemap.setup_pass(ctx);
//   }
// }

// impl<'a> ShaderHashProvider for ForwardSceneLightingDispatcher<'a> {
//   fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
//     self.base.hash_pipeline(hasher);
//     self.lights.light_hash_cache.hash(hasher);
//     self.shadows.hash_pipeline(hasher);

//     self.debugger.is_some().hash(hasher);
//     if let Some(debugger) = &self.debugger {
//       debugger.hash_pipeline(hasher);
//     }

//     self.override_shading.type_id().hash(hasher);
//   }
// }

// impl<'a> ShaderHashProviderAny for ForwardSceneLightingDispatcher<'a> {
//   fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
//     self.hash_pipeline(hasher);
//     // this is so special(I think) that id could skip
//   }
// }

// impl<'a> GraphicsShaderProvider for ForwardSceneLightingDispatcher<'a> {
//   fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
//     self.base.build(builder)
//   }
//   fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError>
// {     self.shadows.build(builder)?;

//     let shading_impl = if let Some(override_shading) = self.override_shading {
//       override_shading
//     } else {
//       *builder
//         .context
//         .entry(ShadingSelection.type_id())
//         .or_insert_with(|| Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn))
//         .downcast_ref::<&dyn LightableSurfaceShadingDyn>()
//         .unwrap()
//     };

//     self.lights.compute_lights(builder, shading_impl)?;

//     self.lighting.tonemap.build(builder)?;

//     builder.fragment(|builder, _| {
//       let ldr = builder.query::<LDRLightResult>()?;

//       let alpha = builder.query::<AlphaChannel>().unwrap_or_else(|_| val(1.0));

//       // should we use other way to get mask mode?
//       let alpha = if builder.query::<AlphaCutChannel>().is_ok() {
//         if_by(alpha.equals(val(0.)), || builder.discard());
//         val(1.)
//       } else {
//         alpha
//       };

//       builder.store_fragment_out(0, (ldr, alpha))
//     })?;

//     if let Some(debugger) = &self.debugger {
//       debugger.build(builder)?;
//     }
//     Ok(())
//   }
// }

// only_fragment!(LightCount, u32);

// impl ForwardLightingSystem {
//   pub fn compute_lights(
//     &self,
//     builder: &mut ShaderRenderPipelineBuilder,
//     shading_impl: &dyn LightableSurfaceShadingDyn,
//   ) -> Result<(), ShaderBuildError> {
//     builder.fragment(|builder, binding| {
//       let lengths_info = binding.bind_by(&self.lengths);
//       let camera_position = builder.query::<CameraWorldMatrix>()?.position();
//       let position =
//         builder.query_or_interpolate_by::<FragmentWorldPosition, WorldVertexPosition>();
//       let normal = builder.get_or_compute_fragment_normal();

//       let geom_ctx = ENode::<ShaderLightingGeometricCtx> {
//         position,
//         normal,
//         view_dir: (camera_position - position).normalize(),
//       };
//       let shading = shading_impl.construct_shading_dyn(builder);

//       let mut light_specular_result = val(Vec3::zero());
//       let mut light_diffuse_result = val(Vec3::zero());

//       for (i, lid) in self.lights_insert_order.iter().enumerate() {
//         let lights = self.lights_collections.get(lid).unwrap();
//         let length = lengths_info.index(val(i as u32)).load().x();
//         builder.register::<LightCount>(length);

//         let ENode::<ShaderLightingResult> { diffuse, specular } = lights
//           .as_ref()
//           .as_ref()
//           .compute_lights(builder, binding, shading_impl, shading.as_ref(), &geom_ctx);
//         light_specular_result = specular + light_specular_result;
//         light_diffuse_result = diffuse + light_diffuse_result;
//       }

//       builder.register::<HDRLightResult>(light_diffuse_result + light_specular_result);

//       Ok(())
//     })
//   }
// }
