use rendiation_shader_api::*;
use rendiation_texture_core::*;
use rendiation_texture_gpu_base::*;
use rendiation_webgpu::*;

mod weighted;
pub use weighted::*;

mod loop32;
pub use loop32::*;

// todo, use SceneRendererPassContentSource
/// this trait is a workaround for lifetime issue, act as a closure
pub trait TransparentPassContentProvider {
  fn get_pass_content<'a>(
    &'a mut self,
    camera: &'a dyn RenderComponent,
    dispatcher: &'a dyn RenderComponent,
  ) -> Box<dyn PassContent + 'a>;
}
