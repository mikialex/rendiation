use rendiation_ral::ResourceManager;
use rendiation_render_entity::Camera;
use rendiation_scenegraph::{Scene, SceneDrawcallList};
use rendiation_webgl::WebGLRenderer;

use crate::GFX;

pub struct NyxtViewerInner {
  pub renderer: WebGLRenderer,
  pub resource: ResourceManager<GFX>,
  pub scene: Scene<GFX>,
  cached_drawcall_list: SceneDrawcallList<GFX>,
  pub camera: Camera,
}

impl NyxtViewerInnerTrait for NyxtViewerInner {
  fn get_uniform<T: Copy + 'static>(&self, handle: UniformHandleWrap<T>) -> &T {
    self.resource.bindable.uniform_buffers.get_data(handle.0)
  }
  fn get_uniform_mut<T: Copy + 'static>(&self, handle: UniformHandleWrap<T>) -> &mut T {
    self.resource.bindable.uniform_buffers.get_data(handle.0)
  }
  fn delete_uniform<T: Copy + 'static>(&self, handle: UniformHandleWrap<T>) {
    self.resource.bindable.uniform_buffers.delete(handle.0);
  }

  fn get_bindgroup<T: BindGroupProvider<GFX>>(
    &self,
    handle: BindGroupHandleWrap<T>,
  ) -> &T::Instance {
    self.resource.bindgroups.get_bindgroup_unwrap(handle.0)
  }
  fn get_bindgroup_mut<T: BindGroupProvider<GFX>>(
    &self,
    handle: BindGroupHandleWrap<T>,
  ) -> &mut T::Instance {
    self.resource.bindgroups.update(handle.0)
  }
  fn delete_bindgroup<T: BindGroupProvider<GFX>>(&self, handle: BindGroupHandleWrap<T>) {
    self.resource.bindgroups.delete(handle.0)
  }

  fn get_shading<T: ShadingProvider<GFX>>(&self, handle: ShadingHandleWrap<T>) -> &T::Instance {
    &self.resource.shadings.get_shading(handle.0).data
  }
  fn get_shading_mut<T: ShadingProvider<GFX>>(
    &self,
    handle: ShadingHandleWrap<T>,
  ) -> &mut T::Instance {
    self.resource.shadings.update_shading(handle.0)
  }
  fn delete_shading<T: ShadingProvider<GFX>>(&self, handle: ShadingHandleWrap<T>) {
    self.resource.shadings.delete_shading(handle.0)
  }
}
