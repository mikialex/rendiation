use crate::*;

pub struct GLESRenderSystem {
  pub model_render_support: Vec<Box<dyn RenderImplProvider<Box<dyn GLESSceneModelRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn SceneRenderer>> for GLESRenderSystem {
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    for imp in &self.model_render_support {
      imp.register_resource(res);
    }
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn SceneRenderer> {
    Box::new(GLESSceneRenderer {
      scene_model_renderer: self
        .model_render_support
        .iter()
        .map(|imp| imp.create_impl(res))
        .collect(),
    })
  }
}

struct GLESSceneRenderer {
  scene_model_renderer: Vec<Box<dyn GLESSceneModelRenderImpl>>,
}

impl SceneRenderer for GLESSceneRenderer {
  fn render(&self, scene: AllocIdx<SceneEntity>) -> Box<dyn FrameContent> {
    todo!()
  }
}

struct GLESSceneRenderContent<'a, T, I> {
  pub scene_models: &'a T,
  pub render_impl: &'a I,
}

impl<'a, T, I> PassContentWithCamera for GLESSceneRenderContent<'a, T, I>
where
  &'a T: IntoIterator<Item = AllocIdx<SceneModelEntity>>,
  I: GLESSceneModelRenderImpl,
{
  fn render(&mut self, pass: &mut FrameRenderPass, camera: AllocIdx<SceneCameraEntity>) {
    for model_idx in self.scene_models {
      //   self.render_impl.make_component(idx, camera, pass)
      //   RenderEmitter::new(contents).emit();
    }
    todo!()
  }
}

// pub struct GLESStyleGPUResourceSystem {
//   pub texture2ds: Box<dyn ReactiveCollection<AllocIdx<SceneTexture2dEntity>, GPU2DTextureView>>,
//   pub texture_cubes: CubeMapUpdateContainer<SceneTextureCubeEntity>,
//   pub samplers: Box<dyn ReactiveCollection<AllocIdx<SceneSamplerEntity>, GPUSamplerView>>,

//   pub attribute_mesh_index_buffers:
//     Box<dyn ReactiveCollection<AllocIdx<AttributeMeshEntity>, GPUBufferResourceView>>,
//   pub attribute_mesh_vertex_buffers:
//     Box<dyn ReactiveCollection<AllocIdx<AttributeMeshVertexBufferRelation>,
// GPUBufferResourceView>>,

//   pub flat_material: FlatMaterialUniforms,
//   pub mr_material: PbrMRMaterialUniforms,

//   pub nodes: SceneNodeUniforms,
//   pub cameras: CameraUniforms,

//   pub directional_lights: UniformArrayUpdateContainer<DirectionalLightUniform>,
//   pub points_lights: UniformArrayUpdateContainer<PointLightUniform>,
//   pub spot_lights: UniformArrayUpdateContainer<SpotLightUniform>,
//   pub foreign_support: FastHashMap<TypeId, Box<dyn Any>>,
// }

// impl GLESStyleGPUResourceSystem {
//   pub fn new(cx: &GPUResourceCtx) -> Self {
//     Self {
//       texture2ds: gpu_texture_2ds(cx).into_boxed(),
//       texture_cubes: gpu_texture_cubes(cx),
//       samplers: sampler_gpus(cx).into_boxed(),
//       directional_lights: directional_uniform_array(cx),
//       points_lights: point_uniform_array(cx),
//       spot_lights: spot_uniform_array(cx),
//       foreign_support: Default::default(),
//       nodes: node_gpus(cx),
//       cameras: todo!(),
//       attribute_mesh_index_buffers: attribute_mesh_index_buffers(cx).into_boxed(),
//       attribute_mesh_vertex_buffers: attribute_mesh_vertex_buffer_views(cx).into_boxed(),
//       flat_material: flat_material_uniforms(cx),
//       mr_material: pbr_mr_material_uniforms(cx),
//     }
//   }

//   pub fn poll_updates(&self, cx: &mut Context) -> GLESStyleGPUResourceSystemAccessView {
//     let _ = self.texture2ds.poll_changes(cx);
//     let _ = self.samplers.poll_changes(cx);

//     GLESStyleGPUResourceSystemAccessView {
//       texture2ds: self.texture2ds.access(),
//       resource: self,
//       foreign_support: Default::default(),
//     }
//   }
// }

// pub struct GLESStyleGPUResourceSystemAccessView<'a> {
//   pub texture2ds: Box<dyn VirtualCollection<AllocIdx<SceneTexture2dEntity>, GPU2DTextureView>>,
//   pub resource: &'a GLESStyleGPUResourceSystem,
//   pub foreign_support: FastHashMap<TypeId, Box<dyn Any>>,
// }

// struct GLESStyleDrawSceneContents<'a> {
//   resource: GLESStyleGPUResourceSystemAccessView<'a>,
//   render_impl: &'a dyn GLESStyleDrawModelMethodProvider,
//   scene_models: Vec<AllocIdx<SceneModelEntity>>,
// }

// pub trait GLESStyleRenderComponentProvider {
//   fn make_component(
//     &self,
//     idx: AllocIdx<SceneModelEntity>,
//     res: GLESStyleGPUResourceSystemAccessView,
//   ) -> Option<Box<dyn RenderComponent>>;
// }

// pub trait GLESStyleDrawModelMethodProvider {
//   fn shape_renderable(
//     &self,
//     idx: AllocIdx<SceneModelEntity>,
//     res: GLESStyleGPUResourceSystemAccessView,
//   ) -> Option<Box<dyn RenderComponent>>;
//   fn material_renderable(
//     &self,
//     idx: AllocIdx<SceneModelEntity>,
//     res: GLESStyleGPUResourceSystemAccessView,
//   ) -> Option<Box<dyn RenderComponent>>;
//   fn draw_command(
//     &self,
//     idx: AllocIdx<SceneModelEntity>,
//     res: GLESStyleGPUResourceSystemAccessView,
//   ) -> Option<DrawCommand>;
// }

// impl<'a> PassContent for GLESStyleDrawSceneContents<'a> {
//   fn render(&mut self, pass: &mut FrameRenderPass) {
//     todo!()
//   }
// }
