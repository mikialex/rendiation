use rendiation_ral::ShaderBindableResourceManager;
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
    todo!()
  }
  fn upload(
    &mut self,
    value: &WebGLTexture,
    renderer: &WebGLRenderer,
    resource: &ShaderBindableResourceManager<WebGLRenderer>,
  ) {
    // renderer.texture_slot_states.get_free_slot()
    todo!()
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
  fn upload(
    &mut self,
    _: &(),
    _: &WebGLRenderer,
    _resource: &ShaderBindableResourceManager<WebGLRenderer>,
  ) {
  }
}
