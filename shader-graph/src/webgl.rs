use rendiation_ral::ResourceManager;
use rendiation_webgl::*;

use crate::{ShaderGraphSampler, ShaderGraphTexture};

impl WebGLUniformUploadable for ShaderGraphTexture {
  type UploadValue = i32;
  type UploadInstance = SingleUniformUploadInstance<i32>;
}

pub struct EmptyImpl;

impl<T: WebGLUniformUploadable> UploadInstance<T> for EmptyImpl {
  fn create(_: &str, _: &WebGl2RenderingContext, _: &WebGlProgram) -> Self {
    Self
  }
  fn upload(
    &mut self,
    _: &T::UploadValue,
    _: &WebGl2RenderingContext,
    _resource: &ResourceManager<WebGLRenderer>,
  ) {
  }
}

impl WebGLUniformUploadable for ShaderGraphSampler {
  type UploadValue = ();
  type UploadInstance = EmptyImpl;
}
