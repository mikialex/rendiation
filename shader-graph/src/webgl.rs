use rendiation_ral::ResourceManager;
use rendiation_webgl::*;

use crate::{ShaderGraphSampler, ShaderGraphTexture};

impl WebGLUniformUploadable for ShaderGraphTexture {
  type UploadValue = WebGLTexture;
  type UploadInstance = TextureUniformUploader;
}

pub struct TextureUniformUploader {
  instance: SingleUniformUploadInstance<i32>,
}

impl UploadInstance<ShaderGraphTexture> for TextureUniformUploader {
  fn create(query_name_prefix: &str, gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Self {
    Self {
      instance: SingleUniformUploadInstance::<i32>::new(query_name_prefix, gl, program),
    }
  }
  fn upload(
    &mut self,
    value: &WebGLTexture,
    renderer: &mut WebGLRenderer,
    _resource: &ResourceManager<WebGL>,
  ) {
    let slot = renderer
      .texture_slot_states
      .bind_and_active_texture(value, &renderer.gl);
    self.instance.upload(&(slot as i32), renderer)
  }
}

impl WebGLUniformUploadable for ShaderGraphSampler {
  type UploadValue = ();
  type UploadInstance = EmptyImpl;
}

pub struct EmptyImpl;

impl UploadInstance<ShaderGraphSampler> for EmptyImpl {
  fn create(_: &str, _: &WebGl2RenderingContext, _: &WebGlProgram) -> Self {
    Self
  }
  fn upload(&mut self, _: &(), _: &mut WebGLRenderer, _resource: &ResourceManager<WebGL>) {}
}
